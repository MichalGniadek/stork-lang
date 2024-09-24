use bevy_ecs::{
    component::Tick,
    system::{SystemMeta, SystemParam},
    world::unsafe_world_cell::UnsafeWorldCell,
};

pub struct UnsafeWorldCellParam<'world>(pub UnsafeWorldCell<'world>);

unsafe impl SystemParam for UnsafeWorldCellParam<'_> {
    type State = ();

    type Item<'world, 'state> = UnsafeWorldCellParam<'world>;

    fn init_state(
        _world: &mut bevy_ecs::world::World,
        _system_meta: &mut SystemMeta,
    ) -> Self::State {
    }

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        UnsafeWorldCellParam(world)
    }
}
