// use bevy::{
//     prelude::*,
//     sprite::{MaterialMesh2dBundle, Mesh2dHandle},
// };

// fn main() {
//     let mut app = App::new();
//     app.add_plugins(DefaultPlugins)
//         .add_systems(Startup, (setup, scripts::setup).chain())
//         .add_systems(Update, scripts::update);
//     app.run();
// }

// fn setup(
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<ColorMaterial>>,
// ) {
//     commands.spawn(Camera2dBundle::default());

//     commands.spawn(MaterialMesh2dBundle {
//         mesh: Mesh2dHandle(meshes.add(Circle::new(50.0))),
//         material: materials.add(Color::hsl(0.5, 0.95, 0.7)),
//         transform: Transform::from_xyz(0.0, 0.0, 0.0),
//         ..default()
//     });
// }
// mod scripts {
//     use bevy::prelude::World;
//     use stork_script::{hir::Module, vm::module_index::ModuleIndex};

//     pub fn setup(world: &mut World) {
//         let mut index = ModuleIndex::default();
//         index
//             .add_module("main", |module_id| {
//                 Module::from_source(include_str!("example.strk"), module_id)
//             })
//             .unwrap();
//         index.add_std(world);
//         let _ = index.compile();
//         index.init_world(world);

//         let system = world
//             .resource::<ModuleIndex>()
//             .get_system_id("main", "startup");

//         world.run_system(system).unwrap();
//     }

//     pub fn update(world: &mut World) {
//         let system = world
//             .resource::<ModuleIndex>()
//             .get_system_id("main", "update");

//         world.run_system(system).unwrap();
//     }
// }
fn main() {}
