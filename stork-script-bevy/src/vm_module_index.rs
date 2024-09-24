use std::sync::RwLock;

use anyhow::bail;
use bevy_ecs::{
    component::ComponentId,
    entity::Entity,
    query::QueryState,
    system::{Resource, SystemId},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

use stork_script_core::{
    hir::{GlobalIdx, Identifier},
    module_index::{cache::GlobalMap, ModuleIndex},
};

use super::{passes, stork_std, StorkValue};

pub type ComponentIdMap = GlobalMap<ComponentId>;
pub type VariableMap = GlobalMap<StorkValue>;
// TODO: remove, and instead pass the queries gotten from system params...
pub type QueryStateMap = GlobalMap<RwLock<QueryState<Entity>>>;
pub type SystemMap = GlobalMap<SystemId>;

#[derive(Default)]
pub struct VMCache {
    pub component_ids: ComponentIdMap,
    pub variables: VariableMap,
    pub query_states: QueryStateMap,
    pub systems: SystemMap,
}

#[derive(Default, Resource)]
pub struct VMModuleIndex {
    pub index: ModuleIndex,
    pub vm_cache: VMCache,
}

impl VMModuleIndex {
    pub fn add_std(&mut self, world: &mut World) {
        let type_registry = &world
            .get_resource::<bevy_ecs::prelude::AppTypeRegistry>()
            .unwrap()
            .read();
        self.index
            .add_module("std".to_string(), |_| {
                Ok(stork_std::new_module(type_registry))
            })
            .unwrap();
    }

    pub fn compile(&mut self, world: &mut World) -> anyhow::Result<()> {
        self.index.compile()?;

        if self.index.has_errors() {
            bail!("There were errors during compilation");
        }

        for module_id in self.index.modules.all_ids() {
            passes::component_id_init::run(
                &mut self.vm_cache,
                &self.index.modules,
                module_id,
                world,
            );
        }
        for module_id in self.index.modules.all_ids() {
            passes::system_init::run(
                &mut self.index.cache,
                &mut self.vm_cache,
                &self.index.modules,
                module_id,
                world,
            );
        }

        Ok(())
    }

    pub fn run_system(&self, system_id: GlobalIdx, world: UnsafeWorldCell) {
        passes::tree_walker::run(
            &self.index.cache,
            &self.vm_cache,
            &self.index.modules,
            system_id,
            world,
        );
    }

    pub fn get_system_id(&self, path: &str, name: &str) -> SystemId {
        let module_id = self.index.modules.path_to_id(path);
        let idx =
            self.index.modules.top_level_names(module_id)[&Identifier::Name(name.to_string())];
        self.vm_cache.systems.get((module_id, idx)).unwrap()
    }
}
