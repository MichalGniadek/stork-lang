use std::any::TypeId;

use crate::hir::{Identifier, Node, Operator};
use crate::module_index::Module;
use crate::passes::type_resolution::{InnerResolvedType, ResolvedType};
use bevy_ecs::reflect::{ReflectComponent, ReflectResource};
use bevy_reflect::func::IntoFunction;
use bevy_reflect::{TypeInfo, TypeRegistry};

pub fn new_module(type_registry: &TypeRegistry) -> Module {
    let mut module = Module {
        parser_errors: Default::default(),
        nodes: Default::default(),
        spans: Default::default(),
        source: String::default().into(),
        top_level: Default::default(),
    };

    for (identifier, logic) in [
        (
            Operator::Add.into(),
            (|a: f32, b: f32| a + b).into_function(),
        ),
        (
            Operator::Sub.into(),
            (|a: f32, b: f32| a - b).into_function(),
        ),
        (
            Operator::Mul.into(),
            (|a: f32, b: f32| a * b).into_function(),
        ),
        (
            Operator::Div.into(),
            (|a: f32, b: f32| a / b).into_function(),
        ),
        (Operator::Neg.into(), (|a: f32| -a).into_function()),
        (
            Operator::Eq.into(),
            (|a: f32, b: f32| a == b).into_function(),
        ),
        (
            Operator::Less.into(),
            (|a: f32, b: f32| a < b).into_function(),
        ),
        (
            Operator::LessEq.into(),
            (|a: f32, b: f32| a <= b).into_function(),
        ),
        (
            Operator::Greater.into(),
            (|a: f32, b: f32| a > b).into_function(),
        ),
        (
            Operator::GreaterEq.into(),
            (|a: f32, b: f32| a >= b).into_function(),
        ),
        (Operator::Not.into(), (|a: bool| !a).into_function()),
        (
            Operator::Or.into(),
            (|a: bool, b: bool| a || b).into_function(),
        ),
        (
            Operator::And.into(),
            (|a: bool, b: bool| a && b).into_function(),
        ),
        ("print".into(), (|a: f32| println!("{a}")).into_function()),
    ] {
        let info = logic.info();
        let r#type = InnerResolvedType::Function {
            params: info
                .args()
                .iter()
                .map(|info| resolve_type(info.type_id(), type_registry).unwrap())
                .collect(),
            ret: Box::new(resolve_type(info.return_info().type_id(), type_registry).unwrap()),
        };

        module.alloc_top_level(Node::BuiltinFunction {
            identifier,
            r#type: r#type.into(),
            effects: Default::default(),
            logic,
        });
    }

    for registration in type_registry.iter() {
        let identifier = Identifier::Name(
            registration
                .type_info()
                .type_path_table()
                .short_path()
                .to_string(),
        );
        if let Some(inner) = resolve_type_info(registration.type_info()) {
            let from_ecs = type_registry
                .get_type_data::<ReflectComponent>(registration.type_id())
                .is_some()
                || type_registry
                    .get_type_data::<ReflectResource>(registration.type_id())
                    .is_some();

            module.alloc_top_level(Node::BuiltinType {
                identifier,
                r#type: ResolvedType { inner, from_ecs },
                effects: Default::default(),
                represents: registration.type_id(),
            });
        }
    }

    module
}

fn resolve_type(type_id: TypeId, type_registry: &TypeRegistry) -> Option<InnerResolvedType> {
    resolve_type_info(type_registry.get_type_info(type_id).unwrap())
}

fn resolve_type_info(type_info: &TypeInfo) -> Option<InnerResolvedType> {
    Some(match type_info {
        TypeInfo::Struct(r#struct) => InnerResolvedType::Struct {
            fields: r#struct
                .iter()
                .filter_map(|f| {
                    Some((
                        f.name().to_string(),
                        resolve_type_info(f.type_info().unwrap())?,
                    ))
                })
                .collect(),
        },
        TypeInfo::Tuple(tuple) if tuple.field_len() == 0 => InnerResolvedType::Unit,
        TypeInfo::Value(info) if info.is::<()>() => InnerResolvedType::Unit,
        TypeInfo::Value(info) if info.is::<f32>() => InnerResolvedType::F32,
        TypeInfo::Value(info) if info.is::<bool>() => InnerResolvedType::Bool,
        TypeInfo::Value(info)
            if info.is::<u8>()
                || info.is::<u16>()
                || info.is::<u32>()
                || info.is::<u64>()
                || info.is::<u128>()
                || info.is::<usize>()
                || info.is::<i8>()
                || info.is::<i16>()
                || info.is::<i32>()
                || info.is::<i64>()
                || info.is::<i128>()
                || info.is::<isize>()
                || info.is::<f64>()
                || info.is::<char>()
                || info.is::<String>() =>
        {
            return None
        }
        _ => {
            return None;
        }
    })
}
