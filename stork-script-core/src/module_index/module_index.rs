pub mod cache;

use std::collections::HashMap;

use anyhow::anyhow;
use ariadne::Source;

use crate::{
    hir::*,
    passes::{self},
    report::Report,
};

use cache::Cache;

#[derive(Default)]
pub struct ModuleIndex {
    pub modules: ModuleCollection,
    pub cache: Cache,
}

impl ModuleIndex {
    pub fn add_module(
        &mut self,
        path: impl Into<String>,
        f: impl FnOnce(ModuleID) -> anyhow::Result<Module>,
    ) -> anyhow::Result<()> {
        let id = self.modules.modules.len();
        self.modules.paths.insert(path.into(), id);
        self.modules.modules.push(f(id)?);
        Ok(())
    }

    pub fn compile(&mut self) -> anyhow::Result<()> {
        self.cache = Cache::default();

        for module_id in self.modules.all_ids() {
            passes::name_resolution::run(&mut self.cache, &self.modules, module_id)?;
        }
        for module_id in self.modules.all_ids() {
            passes::type_resolution::run(&mut self.cache, &self.modules, module_id);
        }
        for module_id in self.modules.all_ids() {
            passes::borrow_resolution::run(&mut self.cache, &self.modules, module_id);
        }
        Ok(())
    }

    pub fn print_errors(&self) {
        for module_id in self.modules.all_ids() {
            for error in &self.modules.get_ref(module_id).parser_errors {
                error.print(&self.modules).unwrap();
            }
            for error in self.cache.errors.get_ref(module_id).unwrap_or(&Vec::new()) {
                error.print(&self.modules).unwrap();
            }
        }
    }

    pub fn has_errors(&self) -> bool {
        self.modules.all_ids().any(|module_id| {
            !self.modules.get_ref(module_id).parser_errors.is_empty()
                || self
                    .cache
                    .errors
                    .get_ref(module_id)
                    .is_some_and(|errors| !errors.is_empty())
        })
    }
}

#[derive(Default)]
pub struct ModuleCollection {
    paths: HashMap<String, usize>,
    modules: Vec<Module>,
}

impl ModuleCollection {
    #[track_caller]
    pub fn get_ref(&self, module_id: ModuleID) -> &Module {
        &self.modules[module_id]
    }

    #[track_caller]
    pub fn get_mut(&mut self, module_id: ModuleID) -> &mut Module {
        self.modules.get_mut(module_id).unwrap()
    }

    #[track_caller]
    pub fn get_node(&self, idx: impl Into<GlobalIdx>) -> &Node {
        let idx = idx.into();
        &self.modules[idx.module()].nodes[idx.idx()]
    }

    pub fn all_ids(&self) -> impl Iterator<Item = usize> {
        0..self.modules.len()
    }

    pub fn path_to_id(&self, path: &str) -> ModuleID {
        self.paths[path]
    }

    pub fn top_level_ids(&self, module_id: ModuleID) -> impl Iterator<Item = GlobalIdx> + Clone {
        self.get_ref(module_id)
            .top_level_ids()
            .map(move |id| (module_id, id).into())
    }

    pub fn top_level_names(&self, module_id: ModuleID) -> HashMap<Identifier, Idx> {
        self.get_ref(module_id).top_level_names()
    }
}

impl ariadne::Cache<ModuleID> for &ModuleCollection {
    type Storage = String;

    fn fetch(
        &mut self,
        id: &ModuleID,
    ) -> std::result::Result<&ariadne::Source<Self::Storage>, Box<dyn std::fmt::Debug + '_>> {
        Ok(&self.modules[*id].source)
    }

    fn display<'a>(&self, id: &'a ModuleID) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(
            self.paths
                .iter()
                .find_map(|(path, module_id)| (id == module_id).then(|| path.clone()))
                .unwrap(),
        ))
    }
}

pub struct Module {
    pub source: Source,
    pub nodes: Arena,
    pub spans: SpanMap,
    pub top_level: Vec<Idx>,
    pub parser_errors: Vec<Report>,
}

impl Module {
    pub fn from_source(source: &str, module_id: ModuleID) -> anyhow::Result<Self> {
        let (cst, errors) = crate::cst::run(source, module_id).map_err(|err| anyhow!("{err:?}"))?;
        let ast = crate::ast::run(cst, module_id).map_err(|err| anyhow!("{err:?}"))?;
        crate::passes::lower::run(ast, source.to_string().into(), module_id, errors)
            .map_err(|err| anyhow!("{err:?}"))
    }

    pub fn alloc_top_level(&mut self, node: Node) {
        let id = self.nodes.alloc(node);
        self.top_level.push(id);
    }
}

impl Module {
    pub fn top_level_ids(&self) -> impl Iterator<Item = Idx> + Clone {
        self.top_level.clone().into_iter()
    }

    pub fn top_level_names(&self) -> HashMap<Identifier, Idx> {
        self.top_level_ids()
            .filter_map(|id| {
                let ident = match &self.nodes[id] {
                    Node::Component(typed_ident) | Node::Resource(typed_ident) => {
                        typed_ident.identifier()
                    }
                    Node::Builtin { identifier, .. } => identifier.clone(),
                    Node::System(System {
                        ident: Some(ident), ..
                    }) => Identifier::Name(ident.clone()),
                    Node::System(_)
                    | Node::TypeIdent(_)
                    | Node::Struct(_)
                    | Node::Expr(_)
                    | Node::Import(_) => {
                        return None;
                    }
                };
                Some((ident, id))
            })
            .collect()
    }
}

pub type ModuleID = usize;
