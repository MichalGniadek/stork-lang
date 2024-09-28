mod name_scope;

use crate::{
    module_index::cache::{ErrorMap, ResolvedDefinition},
    report::{Label, Report, ReportBuilder, ReportKind},
};
use name_scope::NameScope;

use crate::{
    hir::*,
    module_index::{
        cache::{Cache, NameMap},
        ModuleCollection,
    },
};

pub fn run(cache: &mut Cache, modules: &ModuleCollection, module_id: usize) -> anyhow::Result<()> {
    let mut ctx = ResolveCtx {
        errors: &mut cache.errors,
        modules,
        names: &mut cache.names,
        scope: NameScope::new(),
    };

    for node in modules.top_level_ids(module_id) {
        ctx.import(node);
    }
    ctx.import_module(module_id);

    for node in modules.top_level_ids(module_id) {
        ctx.node(node);
    }
    Ok(())
}

struct ResolveCtx<'c> {
    modules: &'c ModuleCollection,
    errors: &'c mut ErrorMap,
    names: &'c mut NameMap,
    scope: NameScope,
}

impl ResolveCtx<'_> {
    fn import(&mut self, item: impl Into<GlobalIdx>) {
        let item = item.into();
        let Node::Import(import) = self.modules.get_node(item) else {
            return;
        };

        self.import_module(self.modules.path_to_id(import));
    }

    fn import_module(&mut self, module_id: usize) {
        for (identifier, idx) in self.modules.get_ref(module_id).top_level_names() {
            self.scope
                .declare(identifier, ResolvedDefinition((module_id, idx).into()));
        }
    }

    fn node(&mut self, node: impl Into<GlobalIdx>) {
        let node = node.into();
        let id = node.module();
        match self.modules.get_node(node) {
            Node::System(system) => {
                if let Some(ident) = &system.ident {
                    self.scope
                        .declare(Identifier::Name(ident.clone()), ResolvedDefinition(node));
                }
                self.node((id, system.block))
            }
            Node::Resource(typed_ident) | Node::Component(typed_ident) => {
                self.node((id, typed_ident.r#type));
            }
            Node::TypeIdent(TypeIdent(identifier)) => {
                if let Some(idx) = self.scope.resolve(&Identifier::Name(identifier.clone())) {
                    self.names.set(node, idx);
                } else {
                    self.errors.push(
                        node.module(),
                        self.error(node)
                            .with_message("Couldn't find name of type")
                            .with_label(self.label(node, "here"))
                            .finish(),
                    );
                }
            }
            Node::Struct(StructType(fields)) => {
                for field_def in fields {
                    self.node((id, field_def.r#type));
                }
            }
            Node::Expr(expr) => self.expr(expr, node),
            Node::Builtin { .. } | Node::Import(_) => {}
        }
    }
}

mod expr {
    use super::*;

    impl ResolveCtx<'_> {
        pub(super) fn expr(&mut self, expr: &Expr, node: GlobalIdx) {
            let id = node.module();
            match &expr {
                Expr::Block(exprs) => {
                    self.scope.push_scope();
                    for expr in exprs {
                        self.node((id, expr));
                    }
                    self.scope.pop_scope();
                }
                Expr::Identifier(name) => {
                    if let Some(resolved) = self.scope.resolve(name) {
                        self.names.set(node, resolved);
                    } else {
                        self.errors.push(
                            node.module(),
                            self.error(node)
                                .with_message("Couldn't find name")
                                .with_label(self.label(node, "here"))
                                .finish(),
                        );
                    }
                }
                Expr::FunctionCall { function, args } => {
                    self.node((id, function));

                    for arg in args {
                        self.node((id, arg));
                    }
                }
                Expr::Query { entity, block } => {
                    self.scope
                        .declare(Identifier::Name(entity.clone()), ResolvedDefinition(node));
                    self.node((id, block));
                }
                Expr::Number(_) | Expr::Poison => {}
                Expr::ComponentAccess { entity, component } => {
                    self.node((id, entity));
                    self.node((id, component));
                }
                Expr::ResourceAccess { resource } => self.node((id, resource)),
                Expr::MemberAccess { base, .. } => {
                    self.node((id, base));
                }
                Expr::Assign { lvalue, expr } => {
                    self.node((id, lvalue));
                    self.node((id, expr));
                }
                Expr::Let { lvalue, expr } => {
                    let lvalue = (id, lvalue).into();
                    if let Some(ident) = self.modules.get_node(lvalue).as_expr_identifier() {
                        self.scope
                            .declare(ident.clone(), ResolvedDefinition(lvalue));
                    } else {
                        self.node(lvalue);
                    }
                    self.node((id, expr));
                }
                Expr::Del { expr } => self.node((id, expr)),
                Expr::If { cond, expr, r#else } => {
                    self.node((id, cond));
                    self.node((id, expr));
                    if let Some(r#else) = r#else {
                        self.node((id, r#else));
                    }
                }
                Expr::While { cond, expr } => {
                    self.node((id, cond));
                    self.node((id, expr));
                }
                Expr::Struct { ident, fields } => {
                    if let Some(resolved) = self.scope.resolve(ident) {
                        self.names.set(node, resolved);
                    } else {
                        self.errors.push(
                            node.module(),
                            self.error(node)
                                .with_message("Couldn't find name of a type")
                                .with_label(self.label(node, "here"))
                                .finish(),
                        );
                    }

                    for (_, field) in fields {
                        self.node((id, field));
                    }
                }
            }
        }
    }
}

impl ResolveCtx<'_> {
    fn error(&self, node: GlobalIdx) -> ReportBuilder {
        Report::build(
            ReportKind::Error,
            node.module(),
            self.modules
                .get_ref(node.module())
                .spans
                .get(node.idx())
                .cloned()
                .map_or_else(Default::default, |ptr| ptr.text_range())
                .start()
                .into(),
        )
    }

    fn label<M: ToString>(&self, node: GlobalIdx, m: M) -> Label {
        Label::new((
            node.module(),
            self.modules
                .get_ref(node.module())
                .spans
                .get(node.idx())
                .copied()
                .map_or_else(Default::default, |ptr| ptr.text_range())
                .into(),
        ))
        .with_message(m)
    }
}
