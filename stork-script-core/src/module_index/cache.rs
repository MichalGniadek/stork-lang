use std::collections::HashMap;

use la_arena::ArenaMap;

use crate::{
    hir::*,
    passes::{borrow_resolution::ResolvedEffects, type_resolution::ResolvedType},
    report::Report,
};

use super::ModuleID;

#[derive(Debug, Clone)]
pub struct GlobalMap<V, K = GlobalIdx>(HashMap<K, V>);

impl<V, K> Default for GlobalMap<V, K> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<V, K: std::cmp::Eq + std::hash::Hash> GlobalMap<V, K> {
    pub fn set(&mut self, idx: impl Into<K>, value: impl Into<V>) {
        self.0.insert(idx.into(), value.into());
    }

    pub fn get(&self, idx: impl Into<K>) -> Option<V>
    where
        V: Clone,
    {
        self.0.get(&idx.into()).cloned()
    }

    pub fn get_ref(&self, idx: impl Into<K>) -> Option<&V> {
        self.0.get(&idx.into())
    }

    pub fn get_mut(&mut self, idx: impl Into<K>) -> Option<&mut V> {
        self.0.get_mut(&idx.into())
    }
}

impl<V> GlobalMap<Vec<V>, ModuleID> {
    pub fn push(&mut self, idx: impl Into<ModuleID>, value: impl Into<V>) {
        self.0.entry(idx.into()).or_default().push(value.into());
    }
}

#[derive(Debug, Clone)]
pub struct DenseGlobalMap<V>(HashMap<ModuleID, ArenaMap<Idx, V>>);

impl<V> Default for DenseGlobalMap<V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<V> DenseGlobalMap<V> {
    pub fn set(&mut self, idx: impl Into<GlobalIdx>, value: impl Into<V>) {
        let idx = idx.into();
        self.0
            .entry(idx.module())
            .or_default()
            .insert(idx.idx(), value.into());
    }

    pub fn get(&self, idx: impl Into<GlobalIdx>) -> Option<V>
    where
        V: Clone,
    {
        let idx = idx.into();
        self.0
            .get(&idx.module())
            .and_then(|module| module.get(idx.idx()))
            .cloned()
    }

    pub fn get_ref(&self, idx: impl Into<GlobalIdx>) -> Option<&V> {
        let idx = idx.into();
        self.0
            .get(&idx.module())
            .and_then(|module| module.get(idx.idx()))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ResolvedDefinition(pub GlobalIdx);

impl ResolvedDefinition {
    pub fn definition(self) -> GlobalIdx {
        self.0
    }
}

pub type NameMap = GlobalMap<ResolvedDefinition>;
// TODO: put types into their own map, and just put the ids here (so you don't duplicate them)
pub type TypeMap = DenseGlobalMap<ResolvedType>;
// TODO: put types into their own map, and just put the ids here (so you don't duplicate them)
pub type EffectMap = GlobalMap<ResolvedEffects>;
pub type ErrorMap = GlobalMap<Vec<Report>, ModuleID>;

#[derive(Default)]
pub struct Cache {
    pub errors: ErrorMap,
    pub names: NameMap,
    pub types: TypeMap,
    pub effects: EffectMap,
}
