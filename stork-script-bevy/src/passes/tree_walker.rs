use std::any::TypeId;

use crate::{
    vm_module_index::{ComponentIdMap, QueryStateMap, VMCache, VariableMap},
    BevyBuiltinData, StorkValue,
};
use bevy_reflect::DynamicStruct;
use itertools::Itertools;
use stork_script_core::{
    hir::*,
    module_index::{
        cache::{Cache, NameMap},
        ModuleCollection,
    },
};

use bevy_ecs::{
    component::{ComponentId, Components},
    entity::Entity,
    ptr::OwningPtr,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy_reflect::{func::ArgList, TypeRegistryArc};

pub fn run(
    cache: &Cache,
    vm_cache: &VMCache,
    modules: &ModuleCollection,
    system_id: GlobalIdx,
    world: UnsafeWorldCell,
) {
    let mut vm = VM {
        modules,
        names: &cache.names,
        component_ids: &vm_cache.component_ids,
        variables: vm_cache.variables.clone(),
        query_states: &vm_cache.query_states,
        registry: unsafe { world.get_resource::<AppTypeRegistry>() }
            .unwrap()
            .0
            .clone(),
        world,
    };

    // TODO: this needs to be always done in exclusive system for structural changes
    vm.node(system_id);
}

struct VM<'a, 'e, 'w> {
    modules: &'a ModuleCollection,
    names: &'a NameMap,
    component_ids: &'a ComponentIdMap,
    variables: VariableMap,
    query_states: &'e QueryStateMap,
    registry: TypeRegistryArc,
    world: UnsafeWorldCell<'w>,
}

impl<'w> VM<'_, '_, 'w> {
    fn node(&mut self, node: impl Into<GlobalIdx>) -> StorkValue {
        let node = node.into();
        let id = node.module();
        match self.modules.get_node(node) {
            Node::System(system) => self.node((id, system.block)),
            Node::Resource(_)
            | Node::Component(_)
            | Node::TypeIdent(_)
            | Node::Struct(_)
            | Node::Builtin { .. }
            | Node::Import(_) => {
                unreachable!()
            }
            Node::Expr(expr) => self.expr(expr, node),
        }
    }

    fn expr(&mut self, expr: &Expr, node: GlobalIdx) -> StorkValue {
        let id = node.module();
        match &expr {
            Expr::Block(exprs) => {
                let mut ret = ().into();
                for expr in exprs {
                    ret = self.node((id, expr));
                }
                ret
            }
            Expr::Identifier(_) => self
                .variables
                .get(self.names.get(node).unwrap().definition())
                .unwrap(),
            Expr::Number(num) => (*num).into(),
            Expr::FunctionCall { function, args } => {
                let f = self.node((id, function)).as_::<(usize, u32)>().unwrap();
                let f = GlobalIdx::construct(f);

                if let Node::Builtin {
                    identifier, data, ..
                } = self.modules.get_node(f)
                {
                    let logic = data
                        .downcast_ref::<BevyBuiltinData>()
                        .unwrap()
                        .unwrap_as_function();
                    let args_values = args
                        .iter()
                        .map(|expr| {
                            if *identifier == Identifier::Operator(Operator::Not) {
                                Box::new(self.node_truthy((id, expr)))
                            } else {
                                self.node((id, expr)).clone_value()
                            }
                        })
                        .collect_vec();

                    let mut args = ArgList::new();
                    for arg_value in args_values {
                        args = args.push_boxed(arg_value);
                    }
                    let ret = logic.call(args).unwrap();
                    if ret.is_unit() {
                        ().into()
                    } else {
                        StorkValue::from_box(ret.unwrap_owned())
                    }
                } else {
                    // TODO: add args to variables
                    self.node(f)
                }
            }
            Expr::Query { block, .. } => {
                // TODO: breaks on function with query recurses
                let mut query = self.query_states.get_ref(node).unwrap().write().unwrap();
                // SAFETY: I don't think this actually needs to be checked if I make sure that
                // the components are not aliased later
                let iter = unsafe { query.iter_unchecked(self.world) };

                for entity in iter {
                    self.variables.set(node, entity);
                    self.node((id, block));
                }

                ().into()
            }
            Expr::Poison => panic!(),
            Expr::ComponentAccess { entity, component } => {
                let var_or_value = self.node((id, entity));
                let Some(entity) = var_or_value.as_::<Entity>() else {
                    panic!()
                };

                let name = self.names.get((id, component)).unwrap().definition();
                let component_id = self.component_ids.get(name).unwrap();

                StorkValue::from_component(entity, component_id, self.world, &self.registry.read())
            }
            Expr::ResourceAccess { resource } => {
                let name = self.names.get((id, resource)).unwrap().definition();
                let component_id = self.component_ids.get(name).unwrap();

                StorkValue::from_resource(component_id, self.world, &self.registry.read())
            }
            Expr::MemberAccess { base, member } => {
                let base = self.node((id, base));

                let Some(Identifier::Name(member)) =
                    self.modules.get_node((id, member)).as_expr_identifier()
                else {
                    panic!();
                };

                base.subscript(Some(member.clone()))
            }
            Expr::Assign { lvalue, expr } => {
                let expr = self.node((id, expr));

                let (lvalue, members) = self.drill_into_member_base((id, lvalue));
                let Node::Expr(lvalue_expr) = self.modules.get_node((id, lvalue)) else {
                    panic!();
                };
                match lvalue_expr {
                    Expr::Identifier(_) => {
                        self.variables
                            .get_mut(self.names.get((id, lvalue)).unwrap().definition())
                            .unwrap()
                            .apply(members, expr);
                    }
                    Expr::ComponentAccess { entity, component } => {
                        let entity = self.node((id, entity));
                        let Some(entity) = entity.as_::<Entity>() else {
                            panic!()
                        };

                        let component_name = self.names.get((id, component)).unwrap().definition();
                        let component_id = self.component_ids.get(component_name).unwrap();

                        expr.apply_to_component(
                            members,
                            entity,
                            component_id,
                            self.world,
                            &self.registry.read(),
                        );
                    }
                    Expr::ResourceAccess { resource } => {
                        let resource_name = self.names.get((id, resource)).unwrap().definition();
                        let component_id = self.component_ids.get(resource_name).unwrap();

                        expr.apply_to_resource(
                            members,
                            component_id,
                            self.world,
                            &self.registry.read(),
                        );
                    }
                    ref kind => unreachable!("{kind:?}"),
                };

                ().into()
            }
            Expr::Let { lvalue, expr } => {
                // TODO: deferred initialization

                // SAFETY: this runs as an exclusive system
                let world = unsafe { self.world.world_mut() };

                let Node::Expr(lvalue_expr) = self.modules.get_node((id, lvalue)) else {
                    panic!();
                };
                match lvalue_expr {
                    Expr::Identifier(_) => {
                        let expr = self.node((id, expr));
                        let expr = expr.clone_value();
                        self.variables.set((id, lvalue), StorkValue::from_box(expr));
                    }
                    Expr::ComponentAccess { entity, component } => {
                        let entity = self.node((id, entity));
                        let Some(entity) = entity.as_::<Entity>() else {
                            panic!()
                        };

                        let expr = self.node((id, expr));
                        let expr = expr;

                        let component_name = self.names.get((id, component)).unwrap().definition();
                        let component_id = self.component_ids.get(component_name).unwrap();
                        let type_id = component_id_to_type_id(world.components(), component_id);

                        if type_id == TypeId::of::<StorkValue>() {
                            OwningPtr::make(expr, |ptr| {
                                unsafe {
                                    world
                                        .get_entity_mut(entity)
                                        .unwrap()
                                        .insert_by_id(component_id, ptr);
                                };
                            });
                        } else {
                            let registry = self.registry.read();
                            let reflect_component =
                                registry.get_type_data::<ReflectComponent>(type_id).unwrap();

                            reflect_component.insert(
                                &mut world.get_entity_mut(entity).unwrap(),
                                expr.clone_value().as_ref(),
                                &self.registry.read(),
                            );
                        }
                    }
                    Expr::ResourceAccess { resource } => {
                        let expr = self.node((id, expr));
                        let expr = expr;

                        let resource_name = self.names.get((id, resource)).unwrap().definition();
                        let component_id = self.component_ids.get(resource_name).unwrap();
                        let type_id = component_id_to_type_id(world.components(), component_id);

                        if type_id == TypeId::of::<StorkValue>() {
                            OwningPtr::make(expr, |ptr| {
                                unsafe {
                                    world.insert_resource_by_id(component_id, ptr);
                                };
                            });
                        } else {
                            let registry = self.registry.read();
                            let reflect_resource =
                                registry.get_type_data::<ReflectResource>(type_id).unwrap();

                            reflect_resource.insert(
                                world,
                                expr.clone_value().as_ref(),
                                &self.registry.read(),
                            );
                        }
                    }
                    ref kind => unreachable!("{kind:?}"),
                };

                ().into()
            }
            Expr::Del { expr } => {
                let world = unsafe { self.world.world_mut() };

                let Node::Expr(lvalue_expr) = self.modules.get_node((id, expr)) else {
                    panic!();
                };
                match lvalue_expr {
                    Expr::ComponentAccess { entity, component } => {
                        let entity = self.node((id, entity));
                        let Some(entity) = entity.as_::<Entity>() else {
                            panic!()
                        };

                        let component_name = self.names.get((id, component)).unwrap().definition();
                        let component_id = self.component_ids.get(component_name).unwrap();

                        world.entity_mut(entity).remove_by_id(component_id);
                    }
                    Expr::ResourceAccess { resource } => {
                        let resource_name = self.names.get((id, resource)).unwrap().definition();
                        let component_id = self.component_ids.get(resource_name).unwrap();

                        world.remove_resource_by_id(component_id);
                    }
                    ref kind => unreachable!("{kind:?}"),
                };

                ().into()
            }
            Expr::If { cond, expr, r#else } => {
                let cond = self.node_truthy((id, cond));

                if cond {
                    self.node((id, expr));
                } else if let Some(r#else) = r#else {
                    self.node((id, r#else));
                }

                ().into()
            }
            Expr::While { cond, expr } => {
                loop {
                    if !self.node_truthy((id, cond)) {
                        break;
                    }
                    self.node((id, expr));
                }

                ().into()
            }
            Expr::Struct { fields, .. } => {
                let mut s = DynamicStruct::default();
                for (name, field) in fields {
                    let value = self.node((id, field)).clone_value();
                    s.insert_boxed(name, value);
                }
                s.into()
            }
        }
    }

    fn drill_into_member_base(&self, idx: impl Into<GlobalIdx>) -> (Idx, Vec<String>) {
        let idx = idx.into();
        let module_id = idx.module();
        let mut idx = idx.idx();
        let mut members = Vec::new();
        while let Node::Expr(Expr::MemberAccess { base, member }) =
            self.modules.get_node((module_id, idx))
        {
            idx = *base;
            let Some(Identifier::Name(member)) = self
                .modules
                .get_node((module_id, member))
                .as_expr_identifier()
            else {
                panic!();
            };

            members.push(member.clone());
        }
        members.reverse();
        (idx, members)
    }

    fn node_truthy(&mut self, idx: impl Into<GlobalIdx>) -> bool {
        let idx = idx.into();
        let id = idx.module();
        match self.modules.get_node(idx) {
            Node::Expr(Expr::ComponentAccess { entity, component }) => {
                let entity = self.node((id, entity));
                let Some(entity) = entity.as_::<Entity>() else {
                    panic!()
                };
                let component_name = self.names.get((id, component)).unwrap().definition();
                let component_id = self.component_ids.get(component_name).unwrap();

                self.world
                    .get_entity(entity)
                    .unwrap()
                    .contains_id(component_id)
            }
            Node::Expr(Expr::ResourceAccess { resource }) => {
                let resource_name = self.names.get((id, resource)).unwrap().definition();
                let component_id = self.component_ids.get(resource_name).unwrap();

                unsafe { self.world.get_resource_by_id(component_id) }.is_some()
            }
            Node::Expr(expr) => self.expr(expr, idx).as_::<bool>().unwrap(),
            _ => panic!(),
        }
    }
}

pub fn component_id_to_type_id(components: &Components, component_id: ComponentId) -> TypeId {
    components
        .get_info(component_id)
        .unwrap()
        .type_id()
        .unwrap()
}
