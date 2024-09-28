use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use bevy_ecs::{
    change_detection::MutUntyped,
    component::{Component, ComponentId},
    entity::Entity,
    ptr::Ptr,
    system::Resource,
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy_reflect::{PartialReflect, ReflectFromPtr, ReflectPath, TypePath, TypeRegistry};

use super::passes::tree_walker::component_id_to_type_id;

// TODO: I think it should be safe to go Arc -> UnsafeCell
// or even maybe just box?
// TODO: yeah so:
// 1. Arc -> Box
// 2. Remove the Clone derive
// 3. Change variable map to have scopes and in the highest scope reference the global variables
#[derive(Debug, Clone, TypePath, Resource, Component)]
pub struct StorkValue(Arc<dyn PartialReflect>, Vec<String>);

impl<T: PartialReflect> From<T> for StorkValue {
    fn from(value: T) -> Self {
        Self(Arc::new(value), Vec::new())
    }
}

impl AsRef<dyn PartialReflect> for StorkValue {
    fn as_ref(&self) -> &dyn PartialReflect {
        self.1
            .join(".")
            .as_str()
            .reflect_element(self.0.as_ref())
            .unwrap()
    }
}

impl AsMut<dyn PartialReflect> for StorkValue {
    fn as_mut(&mut self) -> &mut dyn PartialReflect {
        let refl_mut = Arc::get_mut(&mut self.0).unwrap();
        self.1
            .join(".")
            .as_str()
            .reflect_element_mut(refl_mut)
            .unwrap()
    }
}

impl StorkValue {
    pub fn from_box(val: Box<dyn PartialReflect>) -> Self {
        Self(val.into(), Vec::new())
    }

    pub fn from_component(
        entity: Entity,
        component_id: ComponentId,
        world: UnsafeWorldCell,
        registry: &TypeRegistry,
    ) -> Self {
        let type_id = component_id_to_type_id(world.components(), component_id);
        let entity = world.get_entity(entity).unwrap();
        let ptr = unsafe { entity.get_by_id(component_id) }.unwrap();
        Self::from_box(unsafe { Self::clone_ptr(type_id, ptr, registry) })
    }

    pub fn from_resource(
        component_id: ComponentId,
        world: UnsafeWorldCell,
        registry: &TypeRegistry,
    ) -> Self {
        let type_id = component_id_to_type_id(world.components(), component_id);
        let ptr = unsafe { world.get_resource_by_id(component_id) }.unwrap();
        Self::from_box(unsafe { Self::clone_ptr(type_id, ptr, registry) })
    }

    unsafe fn clone_ptr(
        type_id: TypeId,
        ptr: Ptr,
        registry: &TypeRegistry,
    ) -> Box<dyn PartialReflect> {
        if type_id == TypeId::of::<StorkValue>() {
            ptr.deref::<StorkValue>().clone_value()
        } else {
            let reflect_data = registry.get(type_id).unwrap();
            let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();
            reflect_from_ptr.as_reflect(ptr).clone_value()
        }
    }

    pub fn apply_to_component(
        self,
        members: Vec<String>,
        entity: Entity,
        component_id: ComponentId,
        world: UnsafeWorldCell,
        registry: &TypeRegistry,
    ) {
        let type_id = component_id_to_type_id(world.components(), component_id);
        let entity = world.get_entity(entity).unwrap();
        let ptr = unsafe { entity.get_mut_by_id(component_id) }.unwrap();
        unsafe { self.apply_to_ptr(members, type_id, ptr, registry) }
    }

    pub fn apply_to_resource(
        self,
        members: Vec<String>,
        component_id: ComponentId,
        world: UnsafeWorldCell,
        registry: &TypeRegistry,
    ) {
        let type_id = component_id_to_type_id(world.components(), component_id);
        let ptr = unsafe { world.get_resource_mut_by_id(component_id) }.unwrap();
        unsafe { self.apply_to_ptr(members, type_id, ptr, registry) }
    }

    unsafe fn apply_to_ptr(
        self,
        members: Vec<String>,
        type_id: TypeId,
        mut ptr: MutUntyped,
        registry: &TypeRegistry,
    ) {
        let data = if type_id == TypeId::of::<StorkValue>() {
            ptr.as_mut().deref_mut::<StorkValue>().as_mut()
        } else {
            let reflect_data = registry.get(type_id).unwrap();
            let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();
            reflect_from_ptr
                .as_reflect_mut(ptr.as_mut())
                .as_partial_reflect_mut()
        };
        members
            .join(".")
            .as_str()
            .reflect_element_mut(data)
            .unwrap()
            .apply(self.as_ref());
    }

    pub fn clone_value(&self) -> Box<dyn PartialReflect> {
        self.as_ref().clone_value()
    }

    pub fn as_<T: Any + Clone + std::fmt::Debug>(&self) -> Option<T> {
        self.as_ref().try_downcast_ref::<T>().cloned()
    }

    pub fn apply(&mut self, members: Vec<String>, other: StorkValue) {
        members
            .join(".")
            .as_str()
            .reflect_element_mut(self.as_mut())
            .unwrap()
            .apply(other.as_ref());
    }

    pub fn subscript(mut self, members: impl IntoIterator<Item = String>) -> Self {
        self.1.extend(members);
        self
    }
}
