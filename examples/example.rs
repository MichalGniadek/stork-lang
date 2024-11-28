use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (scripts::compile, startup, scripts::startup).chain(),
        )
        .add_systems(Update, scripts::update);
    app.run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(MaterialMesh2dBundle {
        mesh: Mesh2dHandle(meshes.add(Circle::new(50.0))),
        material: materials.add(Color::hsl(0.5, 0.95, 0.7)),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}
mod scripts {
    use bevy::prelude::World;
    use stork_script_bevy::vm_module_index::VMModuleIndex;
    use stork_script_core::module_index::Module;

    pub fn compile(world: &mut World) {
        world.init_resource::<VMModuleIndex>();
        world.resource_scope::<VMModuleIndex, _>(|mut world, mut vm| {
            vm.index
                .add_module("main", |module_id| {
                    Module::from_source(include_str!("example.strk"), module_id)
                })
                .unwrap();
            vm.add_std(&mut world);
            if let Err(err) = vm.compile(&mut world) {
                vm.index.print_errors();
                panic!("{err}");
            }
        });
    }

    pub fn startup(world: &mut World) {
        let system = world
            .resource::<VMModuleIndex>()
            .get_system_id("main", "startup");

        world.run_system(system).unwrap();
    }

    pub fn update(world: &mut World) {
        let system = world
            .resource::<VMModuleIndex>()
            .get_system_id("main", "update");

        world.run_system(system).unwrap();
    }
}
