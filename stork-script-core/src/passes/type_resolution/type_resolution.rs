mod resolved_type;

use crate::{
    hir::*,
    module_index::{
        cache::{Cache, ErrorMap, NameMap, TypeMap},
        ModuleCollection,
    },
    report::{Label, Report, ReportBuilder, ReportKind},
};
use itertools::{EitherOrBoth, Itertools};
pub use resolved_type::{InnerResolvedType, ResolvedType};

pub fn run(cache: &mut Cache, modules: &ModuleCollection, module_id: usize) {
    let mut ctx = ResolveCtx {
        errors: &mut cache.errors,
        modules,
        names: &cache.names,
        types: &mut cache.types,
    };
    for node in modules.top_level_ids(module_id) {
        ctx.node(node);
    }
}

struct ResolveCtx<'c> {
    errors: &'c mut ErrorMap,
    modules: &'c ModuleCollection,
    names: &'c NameMap,
    types: &'c mut TypeMap,
}

impl ResolveCtx<'_> {
    fn resolve(&mut self, idx: impl Into<GlobalIdx>) -> Option<ResolvedType> {
        let idx = idx.into();
        if let Some(r#type) = self.types.get(idx) {
            if r#type.inner == InnerResolvedType::Recursion {
                self.types.set(idx, InnerResolvedType::Poison);
                self.errors.push(
                    idx.module(),
                    self.error(idx)
                        .with_message("Found recursive type")
                        .with_label(self.label(idx, "here"))
                        .finish(),
                );
                return None;
            }
            if r#type.inner == InnerResolvedType::Poison {
                return None;
            }
            Some(r#type)
        } else {
            // If we don't do this we could recourse infinitely
            // If we terminate at some stop the poison will be overwriten in `fn node()`
            self.types.set(idx, InnerResolvedType::Recursion);
            self.node(idx)
        }
    }
}

impl ResolveCtx<'_> {
    fn node(&mut self, node: impl Into<GlobalIdx>) -> Option<ResolvedType> {
        let node = node.into();
        let mut resolved_type = self.node_inner(node);
        if resolved_type
            .as_ref()
            .is_some_and(|r| r.inner == InnerResolvedType::Poison)
        {
            resolved_type = None;
        }
        self.types
            .set(node, resolved_type.clone().unwrap_or_default());
        resolved_type
    }

    fn node_inner(&mut self, node: GlobalIdx) -> Option<ResolvedType> {
        let id = node.module();
        Some(
            match self.modules.get_node(node) {
                Node::System(system) => {
                    self.node((id, system.block));
                    InnerResolvedType::Poison
                }
                Node::Resource(typed_ident) | Node::Component(typed_ident) => {
                    return Some(self.node((id, typed_ident.r#type))?.with_from_ecs());
                }
                Node::TypeIdent(_) => return self.resolve(self.names.get(node)?.definition()),
                Node::Struct(StructType(fields)) => {
                    let fields = fields
                        .clone()
                        .into_iter()
                        .map(|field_def| {
                            Some((field_def.ident, self.node((id, field_def.r#type))?.inner))
                        })
                        .collect::<Option<_>>()?;
                    InnerResolvedType::Struct { fields }
                }
                Node::Builtin { r#type, .. } => {
                    return Some(r#type.clone());
                }
                Node::Expr(expr) => return self.expr(expr, node),
                Node::Import(_) => InnerResolvedType::Poison,
            }
            .into(),
        )
    }

    fn expr(&mut self, expr: &Expr, node: GlobalIdx) -> Option<ResolvedType> {
        let id = node.module();
        Some(
            match &expr {
                Expr::Block(exprs) => {
                    let mut r#type = InnerResolvedType::Unit.into();
                    for expr in exprs {
                        r#type = self.node((id, expr));
                    }
                    return r#type;
                }
                Expr::Identifier(_) => return self.resolve(self.names.get(node)?.definition()),
                Expr::Number(_) => InnerResolvedType::F32,
                Expr::FunctionCall { function, args } => {
                    let function = (id, *function);
                    let is_not_op = matches!(
                        self.modules.get_node(function),
                        Node::Expr(Expr::Identifier(Identifier::Operator(Operator::Not)))
                    );

                    let (ret, params) = match self.resolve(function)?.inner {
                        InnerResolvedType::Function { ret, params } => (ret, params),
                        r#type => {
                            self.errors.push(
                                node.module(),
                                self.error(node)
                                    .with_message("Call expression requires function")
                                    .with_label(self.label(function, "here"))
                                    .with_note(format!("instead it has type {}", r#type))
                                    .finish(),
                            );
                            return Some(InnerResolvedType::Poison.into());
                        }
                    };

                    let mut already_showed_error = false;
                    for zip in args.iter().zip_longest(&params) {
                        match zip {
                            EitherOrBoth::Both(arg, param) => {
                                let arg = (id, arg);
                                let Some(arg_type) = self.node(arg) else {
                                    continue;
                                };
                                if is_not_op {
                                    self.assert_truthy(arg_type, arg);
                                } else {
                                    self.assert_eq(arg_type, param, arg);
                                }
                            }
                            EitherOrBoth::Left(arg) => {
                                self.node((id, arg));
                                if !already_showed_error {
                                    self.errors.push(
                                        node.module(),
                                        self.error(node)
                                            .with_message("Too many arguments")
                                            .with_label(self.label(node, "in this function call"))
                                            .with_note(format!(
                                                "function takes {} arguments",
                                                params.len()
                                            ))
                                            .finish(),
                                    );
                                }
                                already_showed_error = true;
                            }
                            EitherOrBoth::Right(_) => {
                                self.errors.push(
                                    node.module(),
                                    self.error(node)
                                        .with_message("Too few arguments")
                                        .with_label(self.label(node, "in this function call"))
                                        .with_note(format!(
                                            "function takes {} arguments",
                                            params.len()
                                        ))
                                        .finish(),
                                );
                                break;
                            }
                        }
                    }

                    *ret.clone()
                }
                Expr::Query { block, .. } => {
                    self.types.set(node, InnerResolvedType::Entity);

                    self.node((id, block));
                    InnerResolvedType::Unit
                }
                Expr::Poison => return Some(ResolvedType::poison()),
                Expr::ComponentAccess { entity, component } => {
                    let entity = (id, entity);
                    let entity_type = self.node(entity)?;
                    self.assert_eq(entity_type, &InnerResolvedType::Entity, entity);
                    let component = (id, component);
                    let component_type = self.node(component)?;
                    self.assert_from_ecs(&component_type, component);
                    return Some(component_type);
                }
                Expr::ResourceAccess { resource } => {
                    let resource = (id, resource);
                    let resource_type = self.node(resource)?;
                    self.assert_from_ecs(&resource_type, resource);
                    return Some(resource_type);
                }
                Expr::MemberAccess { base, member } => {
                    let member_idx = (id, member);
                    let Some(Identifier::Name(member)) =
                        self.modules.get_node(member_idx).as_expr_identifier()
                    else {
                        self.errors.push(
                            node.module(),
                            self.error(node)
                                .with_message(
                                    "You can get a field only with a literal string names",
                                )
                                .with_label(self.label(member_idx, "here"))
                                .finish(),
                        );
                        return Some(InnerResolvedType::Poison.into());
                    };

                    let base = self.node((id, base))?;
                    let fields = match base.inner {
                        InnerResolvedType::Struct { fields } => fields,
                        r#type => {
                            self.errors.push(
                                node.module(),
                                self.error(node)
                                    .with_message("Only structs contain fields")
                                    .with_label(self.label(member_idx, "here"))
                                    .with_note(format!("instead it has type {}", r#type))
                                    .finish(),
                            );
                            return Some(InnerResolvedType::Poison.into());
                        }
                    };

                    fields
                        .iter()
                        .find(|(name, _)| name == member)
                        .cloned()
                        .map(|(_, r#type)| r#type)
                        .unwrap_or_default()
                }
                Expr::Assign { lvalue, expr } => {
                    let lvalue_type = self.node((id, lvalue))?;
                    let expr = (id, expr);
                    let expr_type = self.node(expr)?;
                    self.assert_eq(expr_type, &lvalue_type.inner, expr);
                    InnerResolvedType::Unit
                }
                Expr::Let { lvalue, expr } => {
                    let expr = (id, expr);
                    let expr_type = self.node(expr)?;
                    if self
                        .modules
                        .get_node((id, lvalue))
                        .as_expr_identifier()
                        .is_some()
                    {
                        self.types.set((id, lvalue), expr_type);
                    } else {
                        let lvalue = (id, lvalue);
                        let lvalue_type = self.node(lvalue)?;
                        self.assert_from_ecs(&lvalue_type, lvalue);
                        self.assert_eq(expr_type, &lvalue_type.inner, expr);
                    }
                    InnerResolvedType::Unit
                }
                Expr::Del { expr } => {
                    let expr = (id, expr);
                    let expr_type = self.node(expr)?;
                    self.assert_from_ecs(&expr_type, expr);
                    InnerResolvedType::Unit
                }
                // This only returns a unit but only for now...
                Expr::If { cond, expr, r#else } => {
                    let cond = (id, cond);
                    let cond_type = self.node(cond)?;
                    self.assert_truthy(cond_type, cond);

                    self.node((id, expr));
                    if let Some(r#else) = r#else {
                        self.node((id, r#else));
                    }
                    InnerResolvedType::Unit
                }
                Expr::While { cond, expr } => {
                    let cond = (id, cond);
                    let cond_type = self.node(cond)?;
                    self.assert_truthy(cond_type, cond);

                    self.node((id, expr));
                    InnerResolvedType::Unit
                }
                Expr::Struct { fields, .. } => {
                    for (_, field) in fields {
                        self.node((id, field));
                    }
                    return self.resolve(self.names.get(node)?.definition());
                }
            }
            .into(),
        )
    }
}

impl ResolveCtx<'_> {
    fn assert_truthy(&mut self, r#type: ResolvedType, node: impl Into<GlobalIdx>) {
        let node = node.into();
        if r#type.inner != InnerResolvedType::Bool && !r#type.from_ecs {
            self.errors.push(
                node.module(),
                self.error(node)
                    .with_message(format!(
                        "Expected {} or ECS access",
                        InnerResolvedType::Bool
                    ))
                    .with_label(self.label(node, "here"))
                    .with_note(format!("found {}", r#type.inner))
                    .finish(),
            );
        }
    }

    fn assert_eq(&mut self, a: ResolvedType, b: &InnerResolvedType, node: impl Into<GlobalIdx>) {
        let node = node.into();
        if a.inner != *b {
            self.errors.push(
                node.module(),
                self.error(node)
                    .with_message(format!("Expected {}", b))
                    .with_label(self.label(node, "here"))
                    .with_note(format!("found {}", a.inner))
                    .finish(),
            );
        }
    }

    fn assert_from_ecs(&mut self, r#type: &ResolvedType, node: impl Into<GlobalIdx>) {
        let node = node.into();
        if !r#type.from_ecs {
            self.errors.push(
                node.module(),
                self.error(node)
                    .with_message("Type should be a component".to_string())
                    .with_label(self.label(node, "here"))
                    .finish(),
            );
        }
    }

    fn error(&self, node: impl Into<GlobalIdx>) -> ReportBuilder {
        let node = node.into();
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

    fn label<M: ToString>(&self, node: impl Into<GlobalIdx>, m: M) -> Label {
        let node = node.into();
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
