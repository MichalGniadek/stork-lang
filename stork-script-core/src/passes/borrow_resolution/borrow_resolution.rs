mod resolved_effect;

use itertools::{Itertools, Position};
use resolved_effect::join;
pub use resolved_effect::{ComponentEffectKind, ResolvedEffect, ResolvedEffects};

use crate::{
    hir::*,
    module_index::{
        cache::{Cache, EffectMap, ErrorMap, NameMap},
        ModuleCollection,
    },
};

pub fn run(cache: &mut Cache, modules: &ModuleCollection, module_id: usize) {
    let mut ctx = ResolveCtx {
        errors: &mut cache.errors,
        modules,
        names: &cache.names,
        effects: &mut cache.effects,
    };
    for node in modules.top_level_ids(module_id) {
        ctx.node(node, AccessRequirement::None);
    }
}

struct ResolveCtx<'c> {
    modules: &'c ModuleCollection,
    #[expect(unused)]
    errors: &'c mut ErrorMap,
    names: &'c NameMap,
    effects: &'c mut EffectMap,
}

impl ResolveCtx<'_> {
    fn node(
        &mut self,
        node: impl Into<GlobalIdx>,
        ctx: AccessRequirement,
    ) -> Option<ResolvedEffects> {
        let node = node.into();
        let result = self.node_inner(node, ctx);
        self.effects.set(node, result.clone().unwrap_or_default());
        result
    }

    fn node_inner(&mut self, node: GlobalIdx, ctx: AccessRequirement) -> Option<ResolvedEffects> {
        Some(match self.modules.get_node(node) {
            Node::System(system) => return self.node((node.module(), system.block), ctx),
            Node::Resource(_)
            | Node::Component(_)
            | Node::TypeIdent(_)
            | Node::Struct(_)
            | Node::Import(_) => ResolvedEffects::default(),
            Node::Expr(expr) => return self.expr(expr, node, ctx),
            Node::Builtin { effects, .. } => effects.clone(),
        })
    }

    fn expr(
        &mut self,
        expr: &Expr,
        idx: GlobalIdx,
        ctx: AccessRequirement,
    ) -> Option<ResolvedEffects> {
        let id = idx.module();
        match &expr {
            // Hmmm I think because of MVS I don't have to actually validate if the accesses are correct here.
            // Because it will never happen that there are two references to the same value as everything is cloned
            // as soon as it's used anywhere
            // But validating will be needed when:
            // - There are optimizations like cow variables
            // - There are subscripts
            Expr::Query { block, .. } => self.node((id, *block), AccessRequirement::None),
            Expr::Block(exprs) => exprs
                .iter()
                .copied()
                .with_position()
                .map(|(pos, expr)| {
                    let ctx = if [Position::Only, Position::Last].contains(&pos) {
                        ctx
                    } else {
                        AccessRequirement::None
                    };
                    self.node((id, expr), ctx)
                })
                .reduce(join)
                .unwrap_or_default(),
            Expr::FunctionCall { function, args } => {
                // TODO: add function effects
                // TODO: This is AccessContext::Read but only because it's Fn and not FnMut
                let effect = self.node((id, function), AccessRequirement::Read);

                let arg_access =
                    if let Node::Expr(Expr::Identifier(Identifier::Operator(Operator::Not))) =
                        self.modules.get_node((id, function))
                    {
                        AccessRequirement::Has
                    } else {
                        AccessRequirement::Read
                    };

                args.iter()
                    .copied()
                    // TODO: This is AccessContext::Read but only because there are no MVS subscript
                    .map(|expr| self.node((id, expr), arg_access))
                    .fold(effect, join)
            }
            Expr::Identifier(_) | Expr::Number(_) | Expr::Poison => {
                Some(ResolvedEffects::default())
            }
            Expr::ComponentAccess { component, entity } => {
                let component = self.names.get((id, component))?.definition();
                let entity = self.names.get((id, entity))?.definition();
                Some(
                    ctx.component_effect(component, entity)
                        .into_iter()
                        .collect(),
                )
            }
            Expr::ResourceAccess { resource } => {
                let component = self.names.get((id, resource))?.definition();
                Some(ctx.resource_effect(component).into_iter().collect())
            }
            Expr::MemberAccess { base, .. } => {
                let ctx = match ctx {
                    AccessRequirement::None
                    | AccessRequirement::Read
                    | AccessRequirement::Write => ctx,
                    AccessRequirement::Has => AccessRequirement::Read,
                    AccessRequirement::Structural => panic!("Invalid"),
                };
                self.node((id, base), ctx)
            }
            Expr::Assign { lvalue, expr } => join(
                self.node((id, lvalue), AccessRequirement::Write),
                self.node((id, expr), AccessRequirement::Read),
            ),
            Expr::Let { lvalue, expr } => join(
                self.node((id, lvalue), AccessRequirement::Structural),
                self.node((id, expr), AccessRequirement::Read),
            ),
            Expr::Del { expr } => self.node((id, expr), AccessRequirement::Structural),
            Expr::If { cond, expr, r#else } => {
                let mut effects = join(
                    self.node((id, cond), AccessRequirement::Has),
                    // Hm... this is not actually correct. If statement can/should introduce with/without
                    self.node((id, expr), ctx),
                );
                if let Some(r#else) = r#else {
                    effects = join(effects, self.node((id, r#else), ctx));
                }
                effects
            }
            Expr::While { cond, expr } => join(
                self.node((id, cond), AccessRequirement::Has),
                self.node((id, expr), ctx),
            ),
            Expr::Struct { fields, .. } => fields
                .iter()
                .map(|(_, field)| self.node((id, field), AccessRequirement::Read))
                .reduce(join)
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AccessRequirement {
    None,
    Has,
    Read,
    Write,
    Structural,
}

impl AccessRequirement {
    fn component_effect(self, component: GlobalIdx, entity: GlobalIdx) -> Option<ResolvedEffect> {
        match self {
            AccessRequirement::None => None,
            AccessRequirement::Has => Some(ResolvedEffect::Access {
                component,
                kind: ComponentEffectKind::HasComponent { entity },
            }),
            AccessRequirement::Read => Some(ResolvedEffect::Access {
                component,
                kind: ComponentEffectKind::ReadComponent { entity },
            }),
            AccessRequirement::Write => Some(ResolvedEffect::Access {
                component,
                kind: ComponentEffectKind::WriteComponent { entity },
            }),
            AccessRequirement::Structural => Some(ResolvedEffect::Structural {
                entity: Some(entity),
            }),
        }
    }

    fn resource_effect(self, component: GlobalIdx) -> Option<ResolvedEffect> {
        match self {
            AccessRequirement::None => None,
            AccessRequirement::Has => None,
            AccessRequirement::Read => Some(ResolvedEffect::Access {
                component,
                kind: ComponentEffectKind::ReadResource,
            }),
            AccessRequirement::Write => Some(ResolvedEffect::Access {
                component,
                kind: ComponentEffectKind::WriteResource,
            }),
            AccessRequirement::Structural => Some(ResolvedEffect::Structural { entity: None }),
        }
    }
}
