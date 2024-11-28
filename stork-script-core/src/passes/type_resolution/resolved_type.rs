use std::fmt::{Debug, Display};

use itertools::Itertools;

#[derive(Debug, Clone, Default)]
pub struct ResolvedType {
    pub inner: InnerResolvedType,
    pub component_or_resource: bool,
}

impl ResolvedType {
    pub fn poison() -> Self {
        InnerResolvedType::Poison.into()
    }

    pub fn with_from_ecs(mut self) -> Self {
        self.component_or_resource = true;
        self
    }
}

impl From<InnerResolvedType> for ResolvedType {
    fn from(inner: InnerResolvedType) -> Self {
        Self {
            inner,
            ..Default::default()
        }
    }
}

impl From<InnerResolvedType> for Option<ResolvedType> {
    fn from(inner: InnerResolvedType) -> Self {
        Some(ResolvedType {
            inner,
            ..Default::default()
        })
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub enum InnerResolvedType {
    Struct {
        fields: Vec<(String, InnerResolvedType)>,
    },
    Function {
        params: Vec<InnerResolvedType>,
        ret: Box<InnerResolvedType>,
    },
    Entity,
    Unit,
    F32,
    Bool,
    Recursion,
    #[default]
    Poison,
}

impl Debug for InnerResolvedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // TODO: type_path equivalent
            InnerResolvedType::Struct { .. } => f.write_str("InnerResolvedType::Struct"),
            InnerResolvedType::Function { params, ret } => {
                f.write_fmt(format_args!("({params:?}) -> {ret:?}"))
            }
            InnerResolvedType::Entity => f.write_str("InnerResolvedType::Entity"),
            InnerResolvedType::Poison => f.write_str("InnerResolvedType::Poison"),
            InnerResolvedType::Unit => f.write_str("InnerResolvedType::Unit"),
            InnerResolvedType::F32 => f.write_str("InnerResolvedType::F32"),
            InnerResolvedType::Bool => f.write_str("InnerResolvedType::Bool"),
            InnerResolvedType::Recursion => f.write_str("InnerResolvedType::Recursion"),
        }
    }
}

impl Display for InnerResolvedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InnerResolvedType::Struct { .. } => f.write_str("'struct {TODO}'"),
            InnerResolvedType::Function { params, ret } => f.write_fmt(format_args!(
                "'fn({}) -> {ret}'",
                params.iter().map(|p| p.to_string()).format(", "),
            )),
            InnerResolvedType::Entity => f.write_str("'Entity'"),
            InnerResolvedType::Poison => f.write_str("'Unknown'"),
            InnerResolvedType::Unit => f.write_str("'()'"),
            InnerResolvedType::F32 => f.write_str("'f32'"),
            InnerResolvedType::Bool => f.write_str("'bool'"),
            InnerResolvedType::Recursion => f.write_str("INTERNAL"),
        }
    }
}
