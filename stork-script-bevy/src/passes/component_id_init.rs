use crate::{
    vm_module_index::{ComponentIdMap, VMCache, VariableMap},
    BevyBuiltinData, StorkValue,
};
use stork_script_core::{hir::*, module_index::ModuleCollection};

use bevy_ecs::{
    component::ComponentDescriptor,
    world::{FromWorld, World},
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
                // TODO: there is no equivalent to `init_component_with_descriptor` which means:
                //  - we have to do this dumb trick with adding and removing resources
                //  - there can only be one script resource

                impl FromWorld for StorkValue {
                    fn from_world(_: &mut World) -> Self {
                        ().into()
                    }
                }

                let id = self.world.init_resource::<StorkValue>();
                self.world.remove_resource_by_id(id).unwrap();
                self.component_ids.set(node, id);
            }
            Node::Component(_) => {
                let id = self
                    .world
                    .init_component_with_descriptor(ComponentDescriptor::new::<StorkValue>());
                self.component_ids.set(node, id);
            }
            Node::Builtin { data, .. } => {
                match data.downcast_ref::<BevyBuiltinData>().unwrap() {
                    BevyBuiltinData::TypeId(type_id) => {
                        // TODO: extend bevy so that you can get ComponentDescriptor from reflect/type_id
                        // and register component if it doesn't yet exist!
                        // ...ugh wait no, because to register it it would need to be also attached to
                        // type_id (so rust-generic-methods work) which would require adding a weird method
                        // that they probably won't like...
                        // Maybe something like `init_component_by_type_id`?
                        // or bevy#12332
                        if let Some(component_id) = self
                            .world
                            .components()
                            .get_id(*type_id)
                            .or(self.world.components().get_resource_id(*type_id))
                        {
                            self.component_ids.set(node, component_id);
                        }
                    }
                    BevyBuiltinData::Function(_) => {
                        self.variables.set(node, node.destruct());
                    }
                }
            }

            Node::TypeIdent(_) | Node::Struct(_) | Node::Expr(_) | Node::Import(_) => {}
        }
    }
}
