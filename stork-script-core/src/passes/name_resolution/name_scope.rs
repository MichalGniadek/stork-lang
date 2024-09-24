use std::collections::HashMap;

use crate::module_index::cache::ResolvedDefinition;

use super::Identifier;

pub struct NameScope {
    scopes: Vec<HashMap<Identifier, ResolvedDefinition>>,
}

impl NameScope {
    pub fn new() -> Self {
        Self {
            scopes: vec![Default::default()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Default::default());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, name: Identifier, node: impl Into<ResolvedDefinition>) {
        self.scopes.last_mut().unwrap().insert(name, node.into());
    }

    pub fn resolve(&self, name: &Identifier) -> Option<ResolvedDefinition> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    #[expect(dead_code)]
    pub fn into_ident_map(self) -> HashMap<Identifier, ResolvedDefinition> {
        self.scopes
            .into_iter()
            .fold(Default::default(), |mut a, b| {
                a.extend(b);
                a
            })
    }
}
