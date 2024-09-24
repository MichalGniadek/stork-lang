use std::sync::RwLock;

use crate::bevy::vm::utils::UnsafeWorldCellParam;
use crate::bevy::vm::vm_module_index::{
    ComponentIdMap, QueryStateMap, SystemMap, VMCache, VMModuleIndex,
};
use crate::hir::*;
use crate::module_index::cache::{Cache, EffectMap};
use crate::module_index::ModuleCollection;
use crate::passes::borrow_resolution::{ComponentEffectKind, ResolvedEffect, ResolvedEffects};

use bevy_ecs::component::ComponentId;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{FilteredAccess, QueryBuilder};
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::system::{DynParamBuilder, ParamBuilder, ParamSetBuilder, QueryParamBuilder};
use bevy_ecs::{prelude::SystemParamBuilder, world::World};

pub fn run(
    cache: &mut Cache,
    vm_cache: &mut VMCache,
    modules: &ModuleCollection,
    module_id: usize,
    world: &mut World,
) {
    let mut vm = WorldInitCtx {
        modules,
        effects: &cache.effects,
        component_ids: &vm_cache.component_ids,
        query_states: &mut vm_cache.query_states,
        systems: &mut vm_cache.systems,
        world,
        current_system_params: None,
    };

    for node in modules.top_level_ids(module_id) {
        vm.node(node);
    }
}

struct WorldInitCtx<'a, 'e, 'w> {
    modules: &'a ModuleCollection,
    effects: &'a EffectMap,
    component_ids: &'e ComponentIdMap,
    query_states: &'e mut QueryStateMap,
    systems: &'e mut SystemMap,
    world: &'w mut World,
    current_system_params: Option<Vec<DynParamBuilder<'static>>>,
}

impl WorldInitCtx<'_, '_, '_> {
    fn node(&mut self, node: impl Into<GlobalIdx>) {
        let node = node.into();
        let id = node.module();
        match self.modules.get_node(node) {
            Node::System(system) => {
                self.current_system_params = Some(Vec::new());
                self.node((id, system.block));
                let system_params = self.current_system_params.take().unwrap();

                let system = (
                    ParamSetBuilder(system_params),
                    ParamBuilder::resource::<VMModuleIndex>(),
                    ParamBuilder::of::<UnsafeWorldCellParam>(),
                    // For safety as it's accessed later
                    ParamBuilder::resource::<AppTypeRegistry>(),
                )
                    .build_state(self.world)
                    .build_system(move |_, index, world, _| {
                        index.run_system(node, world.0);
                    });

                let id = self.world.register_system(system);
                self.systems.set(node, id);
            }
            Node::Expr(expr) => self.expr(expr, node),
            Node::TypeIdent(_)
            | Node::Struct(_)
            | Node::Import(_)
            | Node::Resource(_)
            | Node::Component(_)
            | Node::BuiltinType { .. }
            | Node::BuiltinFunction { .. } => {}
        }
    }

    fn expr(&mut self, expr: &Expr, node: GlobalIdx) {
        let id = node.module();
        match &expr {
            Expr::Block(exprs) => {
                for expr in exprs {
                    self.node((id, expr));
                }
            }
            Expr::Identifier(_) => {}
            Expr::FunctionCall { function, args } => {
                self.node((id, function));

                for arg in args {
                    self.node((id, arg));
                }
            }
            Expr::Query { entity: _, block } => {
                let effects = self.effects.get_ref(node).unwrap();
                let access = self.effects_to_access(effects);

                let mut builder = QueryBuilder::<Entity>::new(self.world);
                builder.extend_access(access.clone());

                // TODO: this conflicts with &World which is used to get UnsafeWorldCell...
                self.current_system_params
                    .as_mut()
                    .unwrap()
                    .push(DynParamBuilder::new(QueryParamBuilder::new(
                        |b: &mut QueryBuilder<'_, Entity, ()>| b.extend_access(access),
                    )));

                self.query_states.set(node, RwLock::new(builder.build()));

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
                if self
                    .modules
                    .get_node((id, lvalue))
                    .as_expr_identifier()
                    .is_none()
                {
                    self.node((id, lvalue));
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
            Expr::Struct { fields, .. } => {
                for (_, field) in fields {
                    self.node((id, field));
                }
            }
        }
    }
}

impl WorldInitCtx<'_, '_, '_> {
    fn effects_to_access(&self, effects: &ResolvedEffects) -> FilteredAccess<ComponentId> {
        let mut access = FilteredAccess::<ComponentId>::default();

        for effect in effects {
            match effect {
                ResolvedEffect::Access { component, kind } => {
                    let component_id = self.component_ids.get(*component).unwrap();
                    match kind {
                        ComponentEffectKind::ReadComponent { .. } => {
                            access.add_component_read(component_id)
                        }
                        ComponentEffectKind::WriteComponent { .. } => {
                            access.add_component_write(component_id)
                        }
                        ComponentEffectKind::ReadResource => access.add_resource_read(component_id),
                        ComponentEffectKind::WriteResource => {
                            access.add_resource_write(component_id);
                        }
                        ComponentEffectKind::HasComponent { .. } => {
                            // Checking for component existance isn't an access
                        }
                    }
                }
                ResolvedEffect::Structural { entity } =>
                {
                    #[expect(clippy::redundant_pattern_matching)]
                    if let Some(_) = entity {
                        access.access_mut().write_all_components();
                    } else {
                        access.access_mut().write_all_resources();
                    }
                }
            }
        }

        access
    }
}
