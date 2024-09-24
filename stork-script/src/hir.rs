use crate::passes::type_resolution::ResolvedType;
use crate::{module_index::ModuleID, passes::borrow_resolution::ResolvedEffects};
use bevy_reflect::{func::DynamicFunction, Reflect};
use rowan::TextRange;
use std::{any::TypeId, fmt::Debug};

pub type Arena = la_arena::Arena<Node>;
pub type Idx = la_arena::Idx<Node>;

pub type SpanMap = la_arena::ArenaMap<Idx, TextRange>;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Reflect)]
pub struct GlobalIdx(ModuleID, u32);

impl GlobalIdx {
    pub fn new(module: ModuleID, idx: Idx) -> Self {
        Self(module, idx.into_raw().into_u32())
    }

    pub fn from_index(module: ModuleID, i: usize) -> Self {
        Self(module, i as u32)
    }

    pub fn module(&self) -> ModuleID {
        self.0
    }

    pub fn idx(&self) -> Idx {
        crate::hir::Idx::from_raw(la_arena::RawIdx::from_u32(self.1))
    }

    pub fn destruct(&self) -> (ModuleID, Idx) {
        (self.module(), self.idx())
    }
}

impl From<(ModuleID, Idx)> for GlobalIdx {
    fn from((module, idx): (ModuleID, Idx)) -> Self {
        Self::new(module, idx)
    }
}

impl From<(ModuleID, &Idx)> for GlobalIdx {
    fn from((module, idx): (ModuleID, &Idx)) -> Self {
        Self::new(module, *idx)
    }
}

impl Debug for GlobalIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("#{}#{}", self.0, self.1))
    }
}

pub enum Node {
    System(System),
    Resource(Resource),
    Component(Component),
    Import(String),
    TypeIdent(TypeIdent),
    Struct(StructType),
    Expr(Expr),
    BuiltinType {
        identifier: Identifier,
        r#type: ResolvedType,
        effects: ResolvedEffects,
        represents: TypeId,
    },
    BuiltinFunction {
        identifier: Identifier,
        r#type: ResolvedType,
        effects: ResolvedEffects,
        logic: DynamicFunction<'static>,
    },
}

impl From<Expr> for Node {
    fn from(value: Expr) -> Self {
        Self::Expr(value)
    }
}

impl Node {
    pub fn as_expr_identifier(&self) -> Option<&Identifier> {
        if let Node::Expr(Expr::Identifier(ident)) = self {
            Some(ident)
        } else {
            None
        }
    }
}

pub type Resource = TypedIdent;
pub type Component = TypedIdent;

#[derive(Debug, Clone)]
pub struct TypedIdent {
    pub ident: String,
    pub r#type: Idx,
}

impl TypedIdent {
    pub fn identifier(&self) -> Identifier {
        Identifier::Name(self.ident.clone())
    }
}

pub struct TypeIdent(pub String);
pub struct StructType(pub Vec<TypedIdent>);

pub struct System {
    pub ident: Option<String>,
    pub block: Idx,
}

#[derive(Debug)]
pub enum Expr {
    Block(Vec<Idx>),
    Identifier(Identifier),
    Number(f32),
    ComponentAccess {
        entity: Idx,
        component: Idx,
    },
    ResourceAccess {
        resource: Idx,
    },
    MemberAccess {
        base: Idx,
        member: Idx,
    },
    Assign {
        lvalue: Idx,
        expr: Idx,
    },
    FunctionCall {
        function: Idx,
        args: Vec<Idx>,
    },
    Query {
        entity: String,
        block: Idx,
    },
    Let {
        lvalue: Idx,
        expr: Idx,
    },
    Del {
        expr: Idx,
    },
    If {
        cond: Idx,
        expr: Idx,
        r#else: Option<Idx>,
    },
    While {
        cond: Idx,
        expr: Idx,
    },
    Struct {
        ident: Identifier,
        fields: Vec<(String, Idx)>,
    },
    Poison,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Identifier {
    Name(String),
    Operator(Operator),
}

impl From<Operator> for Identifier {
    fn from(value: Operator) -> Self {
        Self::Operator(value)
    }
}

impl<T: Into<String>> From<T> for Identifier {
    fn from(value: T) -> Self {
        Self::Name(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Eq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    Not,
    Or,
    And,
}
