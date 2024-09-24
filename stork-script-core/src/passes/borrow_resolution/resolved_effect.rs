use crate::hir::*;
use std::{collections::HashSet, fmt::Debug};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolvedEffect {
    Access {
        component: GlobalIdx,
        kind: ComponentEffectKind,
    },
    Structural {
        entity: Option<GlobalIdx>,
    },
}

impl Debug for ResolvedEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access { component, kind } => {
                f.debug_tuple("").field(component).field(kind).finish()
            }
            Self::Structural { entity } => f.debug_tuple("Structural").field(entity).finish(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentEffectKind {
    ReadResource,
    WriteResource,
    ReadComponent { entity: GlobalIdx },
    WriteComponent { entity: GlobalIdx },
    HasComponent { entity: GlobalIdx },
}

impl Debug for ComponentEffectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadResource => write!(f, "R-res"),
            Self::WriteResource => write!(f, "W-res"),
            Self::ReadComponent { entity } => f.debug_tuple("R-comp").field(entity).finish(),
            Self::WriteComponent { entity } => f.debug_tuple("W-comp").field(entity).finish(),
            Self::HasComponent { entity } => f.debug_tuple("C-comp").field(entity).finish(),
        }
    }
}

pub type ResolvedEffects = HashSet<ResolvedEffect>;

pub fn join(a: Option<ResolvedEffects>, b: Option<ResolvedEffects>) -> Option<ResolvedEffects> {
    let mut a = a.unwrap_or_default();
    let b = b.unwrap_or_default();
    a.extend(b);
    Some(a)
}
