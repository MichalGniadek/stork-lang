use bevy_ecs::prelude::{ReflectComponent, ReflectResource};
use bevy_ecs::{component::Component, system::Resource};
use bevy_ecs::{reflect::AppTypeRegistry, world::World};
use bevy_reflect::Reflect;
use stork_script_bevy::vm_module_index::VMModuleIndex;
use stork_script_core::module_index::Module;

#[derive(Debug, Reflect, Component, Default, PartialEq)]
#[reflect(Component)]
pub struct Transform {
    pub translation: Translation,
}

#[derive(Debug, Reflect, Default, PartialEq)]
pub struct Translation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Reflect, Resource, Default, PartialEq)]
#[reflect(Resource)]
pub struct Difficulty {
    pub value: f32,
}

fn create_world() -> World {
    let mut world = World::new();
    world.init_resource::<AppTypeRegistry>();

    {
        let mut registry = world.get_resource::<AppTypeRegistry>().unwrap().write();
        registry.register::<()>();
        registry.register::<Transform>();
        registry.register::<Difficulty>();
    }

    world.spawn_empty();
    world.spawn_empty();
    world.spawn(Transform::default());
    world.spawn(Transform::default());
    world.spawn(Transform::default());
    world.insert_resource(Difficulty::default());
    world
}

fn run(source: &str) -> World {
    let mut world = create_world();
    world.init_resource::<VMModuleIndex>();
    world.resource_scope::<VMModuleIndex, _>(|mut world, mut vm| {
        vm.index
            .add_module("main", |module_id| Module::from_source(source, module_id))
            .unwrap();
        vm.add_std(&mut world);
        if let Err(err) = vm.compile(&mut world) {
            vm.index.print_errors();
            panic!("{err}");
        }
    });
    world
}

#[test]
fn simple() {
    let mut world = run("
    use std

    res Health: f32

    sys first_system {
        query entity {
            let v = 1;
            entity[Transform].translation.x = v;
        };
        query entity {
            entity[Transform].translation.y = entity[Transform].translation.x + 3;
        };

        [Difficulty].value = 10
    }
            
    sys second_system {
        let [Health] = 5;
        [Health] = 123;
        print([Health]);
        query entity {
            if [Health] == 123 {
                entity[Transform].translation.z = [Health];
            }
        }
    }
    
    ");

    let first_system = world
        .resource::<VMModuleIndex>()
        .get_system_id("main", "first_system");
    let second_system = world
        .resource::<VMModuleIndex>()
        .get_system_id("main", "second_system");

    world.run_system(first_system).unwrap();
    world.run_system(second_system).unwrap();

    for e in world.iter_entities() {
        if let Some(transform) = e.get::<Transform>() {
            assert_eq!(
                transform,
                &Transform {
                    translation: Translation {
                        x: 1.,
                        y: 4.,
                        z: 123.,
                        ..Default::default()
                    }
                }
            )
        }
    }
    assert_eq!(world.resource::<Difficulty>(), &Difficulty { value: 10. });
}
