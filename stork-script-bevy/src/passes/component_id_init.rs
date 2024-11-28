use crate::{
    vm_module_index::{ComponentIdMap, VMCache, VariableMap},
    BevyBuiltinData, StorkValue,
};
use stork_script_core::{hir::*, module_index::ModuleCollection};

use bevy_ecs::{
    component::ComponentDescriptor,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    world::World,
};

pub fn run(
    vm_cache: &mut VMCache,
    modules: &ModuleCollection,
    module_id: usize,
    world: &mut World,
) {
    let mut vm = WorldInitCtx {
        modules,
        component_ids: &mut vm_cache.component_ids,
        variables: &mut vm_cache.variables,
        world,
    };

    for node in modules.top_level_ids(module_id) {
        vm.node(node);
    }
}

struct WorldInitCtx<'a, 'e, 'w> {
    modules: &'a ModuleCollection,
    component_ids: &'e mut ComponentIdMap,
    variables: &'e mut VariableMap,
    world: &'w mut World,
}

impl WorldInitCtx<'_, '_, '_> {
    fn node(&mut self, node: impl Into<GlobalIdx>) {
        let node = node.into();
        match self.modules.get_node(node) {
            Node::System(_) => {}
            Node::Resource(_) => {
                let id = self
                    .world
                    .register_resource_with_descriptor(ComponentDescriptor::new::<StorkValue>());
                self.component_ids.set(node, id);
            }
            Node::Component(_) => {
                let id = self
                    .world
                    .register_component_with_descriptor(ComponentDescriptor::new::<StorkValue>());
                self.component_ids.set(node, id);
            }
            Node::Builtin { data, .. } => match data.downcast_ref::<BevyBuiltinData>().unwrap() {
                BevyBuiltinData::TypeId(type_id) => {
                    self.world
                        .resource_scope::<AppTypeRegistry, _>(|world, registry| {
                            let registry = registry.read();
                            let component = registry.get_type_data::<ReflectComponent>(*type_id);
                            let resource = registry.get_type_data::<ReflectResource>(*type_id);
                            let id = match (component, resource) {
                                (None, None) => None,
                                (Some(component), None) => {
                                    Some(component.register_component(world))
                                }
                                (None, Some(resource)) => Some(resource.register_resource(world)),
                                (Some(_), Some(_)) => {
                                    todo!("Handle types that are both components and resources")
                                }
                            };
                            if let Some(id) = id {
                                self.component_ids.set(node, id);
                            }
                        });
                }
                BevyBuiltinData::Function(_) => {
                    self.variables.set(node, node.destruct());
                }
            },

            Node::TypeIdent(_) | Node::Struct(_) | Node::Expr(_) | Node::Import(_) => {}
        }
    }
}
