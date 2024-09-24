use itertools::Itertools;

use crate::{
    hir::*,
    module_index::{
        cache::{Cache, EffectMap, NameMap, TypeMap},
        ModuleCollection, ModuleID,
    },
};

pub fn run(cache: &mut Cache, modules: &ModuleCollection, module_id: ModuleID) -> String {
    let print = PrintCtx {
        modules,
        names: &cache.names,
        types: &cache.types,
        effects: &cache.effects,
    };

    format!(
        "Root\n  {}",
        modules
            .top_level_ids(module_id)
            .map(|item| print.node(item, 0))
            .join("\n  ")
    )
}

struct PrintCtx<'a> {
    modules: &'a ModuleCollection,
    names: &'a NameMap,
    types: &'a TypeMap,
    effects: &'a EffectMap,
}

impl PrintCtx<'_> {
    fn node(&self, node: impl Into<GlobalIdx>, indent: usize) -> String {
        let node = node.into();
        let id = node.module();
        match self.modules.get_node(node) {
            Node::System(system) => {
                format!(
                    "System {}\n{}",
                    self.names
                        .get(node)
                        .map_or(String::new(), |name| format!("{name:?}")),
                    self.node((id, system.block), indent + 1),
                )
            }
            Node::Component(typed_ident) => {
                format!(
                    "Component {} {}",
                    self.names
                        .get(node)
                        .map_or(String::new(), |name| format!("{name:?}")),
                    self.print_typed_ident(typed_ident, indent + 1, id)
                )
            }
            Node::Resource(typed_ident) => {
                format!(
                    "Resource {} {}",
                    self.names
                        .get(node)
                        .map_or(String::new(), |name| format!("{name:?}")),
                    self.print_typed_ident(typed_ident, indent + 1, id)
                )
            }
            Node::TypeIdent(TypeIdent(identifier)) => {
                format!(
                    "{} {}",
                    identifier,
                    self.names
                        .get(node)
                        .map_or(String::new(), |name| format!("{name:?}"))
                )
            }
            Node::Struct(StructType(fields)) => format!(
                "Struct\n{}",
                fields
                    .iter()
                    .map(|field| self.print_typed_ident(field, indent + 1, id))
                    .join("\n")
            ),
            Node::Expr(expr) => self.expr(expr, node, indent),
            Node::BuiltinType { .. } => "BuiltinType".to_string(),
            Node::BuiltinFunction { .. } => "BuiltinFunction".to_string(),
            Node::Import(import) => format!("Import {import:?}"),
        }
    }

    fn print_typed_ident(&self, typed_ident: &TypedIdent, indent: usize, id: ModuleID) -> String {
        format!(
            "{}: {}",
            typed_ident.ident,
            self.node((id, typed_ident.r#type), indent + 1)
        )
    }

    fn expr(&self, expr: &Expr, node: GlobalIdx, indent: usize) -> String {
        let id = node.module();
        let t = &self
            .types
            .get(node)
            .map_or(Default::default(), |t| format!(": {t:?}"));
        let e = &self
            .effects
            .get(node)
            .map_or(Default::default(), |e| format!(" / {e:?}"));
        let te = format!("{t}{e}");

        format!(
            "{}{}",
            " ".repeat(indent * 2),
            match &expr {
                Expr::Block(exprs) => format!(
                    "Block{te}\n{}",
                    exprs
                        .iter()
                        .map(|expr| self.node((id, expr), indent + 1))
                        .join("\n")
                ),
                Expr::Identifier(_) => format!(
                    "Identifier{}{te}",
                    self.names
                        .get(node)
                        .map_or(Default::default(), |n| format!(" = {n:?}")),
                ),
                Expr::Number(number) => format!("Number {number:?}{te}"),
                Expr::ComponentAccess { entity, component } => format!(
                    "ComponentAccess{te}\n{}\n{}",
                    self.node((id, entity), indent + 1),
                    self.node((id, component), indent + 1)
                ),
                Expr::ResourceAccess { resource } => format!(
                    "ComponentAccess{te}\n{}",
                    self.node((id, resource), indent + 1),
                ),
                Expr::MemberAccess { base, member } => format!(
                    "MemberAccess{te}\n{}\n{}",
                    self.node((id, base), indent + 1),
                    self.node((id, member), indent + 1)
                ),
                Expr::Assign { lvalue, expr } => format!(
                    "Assign{te}\n{}\n{}",
                    self.node((id, lvalue), indent + 1),
                    self.node((id, expr), indent + 1)
                ),
                Expr::FunctionCall { function, args } => format!(
                    "FunctionCall{te}\n{}\n{}",
                    self.node((id, function), indent + 1),
                    args.iter()
                        .map(|expr| self.node((id, expr), indent + 1))
                        .join("\n")
                ),
                Expr::Query { block, .. } => format!(
                    "Query {:?}{te}\n{}",
                    self.names.get(node),
                    self.node((id, block), indent + 1)
                ),
                Expr::Let { lvalue, expr } => format!(
                    "Let {te}\n{}\n{}",
                    self.node((id, lvalue), indent + 1),
                    self.node((id, expr), indent + 1)
                ),
                Expr::Del { expr } => format!("Del {te}\n{}\n", self.node((id, expr), indent + 1)),
                Expr::If { cond, expr, r#else } => format!(
                    "If {te}\n{}\n{}{}",
                    self.node((id, cond), indent + 1),
                    self.node((id, expr), indent + 1),
                    r#else.map_or(String::new(), |r#else| format!(
                        "\n{}",
                        self.node((id, r#else), indent + 1)
                    )),
                ),
                Expr::While { cond, expr } => format!(
                    "While {te}\n{}\n{}",
                    self.node((id, cond), indent + 1),
                    self.node((id, expr), indent + 1),
                ),
                Expr::Struct { ident, fields } => format!(
                    "Struct {te}\n{:?}\n{}",
                    ident,
                    fields
                        .iter()
                        .map(|(member, expr)| format!(
                            "{member}: {}",
                            self.node((id, expr), indent + 1)
                        ))
                        .join("\n")
                ),
                Expr::Poison => "POISON".to_string(),
            }
        )
    }
}
