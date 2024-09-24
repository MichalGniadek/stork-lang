use std::any::TypeId;

use bevy_reflect::func::DynamicFunction;

#[path = "passes/passes.rs"]
mod passes;
pub mod stork_std;
pub mod stork_value;
mod utils;
pub mod vm_module_index;

pub use stork_value::*;

enum BevyBuiltinData {
    TypeId(TypeId),
    Function(DynamicFunction<'static>),
}

impl BevyBuiltinData {
    pub fn unwrap_as_function(&self) -> &DynamicFunction<'static> {
        match self {
            BevyBuiltinData::TypeId(_) => panic!(),
            BevyBuiltinData::Function(function) => function,
        }
    }
}
