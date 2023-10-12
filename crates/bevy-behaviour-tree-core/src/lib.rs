//! bevy-behaviour-tree is a crate for defining simple, composable, and extensible behaviour trees for [bevy].
#![warn(missing_docs)]
#![feature(return_position_impl_trait_in_trait)]

/// Basic [`Behaviour`][behaviour::Behaviour] trait and impls.
pub mod behaviour;
/// Compositor behaviour impls.
pub mod compositor;
/// Decorator behaviour impls.
pub mod decorator;
/// The actual plugin and related stuff.
pub mod plugin;

/// Quick imports!
///
/// Best used as `use bevy_behaviour_tree::prelude::*`.
pub mod prelude {
    pub use super::behaviour::{Behaviour, Status};
    pub use super::compositor::Compositor;
    pub use super::decorator::Decorator;
    pub use super::plugin::{BehaviourId, BehaviourTreePlugin, BehaviourTrees};
}

/// For debug purposes only. Panics if used in any way.
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct TodoBehaviour;

impl behaviour::Behaviour for TodoBehaviour {
    fn initialize(&mut self, _: &mut bevy::prelude::World) {
        todo!()
    }

    fn run(&mut self, _: bevy::prelude::Entity, _: &mut bevy::prelude::World) -> behaviour::Status {
        todo!()
    }
}
#[cfg(test)]
mod tests {
    use bevy::prelude::{Component, Entity, In, IntoSystem, Query, World};

    use crate::prelude::*;

    fn succeed(In(_): In<Entity>) -> Status {
        Status::Success
    }

    fn fail(In(_): In<Entity>) -> Status {
        Status::Failure
    }

    fn panic_if_run(_: In<Entity>) -> Status {
        panic!(":(");
    }

    #[test]
    fn test_invert() {
        let mut world = World::default();

        let mut system = IntoSystem::into_system(succeed).invert();

        let entity = world.spawn_empty().id();

        Behaviour::initialize(&mut system, &mut world);

        let status = Behaviour::run(&mut system, entity, &mut world);

        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_retry() {
        let mut world = World::default();

        #[derive(Component)]
        struct Counter(u32);

        let system = IntoSystem::into_system(
            move |In(entity): In<Entity>, mut counters: Query<&mut Counter>| {
                let mut counter = counters.get_mut(entity).unwrap();
                counter.0 += 1;

                if counter.0 < 3 {
                    Status::Failure
                } else {
                    Status::Success
                }
            },
        );

        let mut retry = system.retry(5);

        retry.initialize(&mut world);

        let entity = world.spawn(Counter(0)).id();

        while let Status::Running = retry.run(entity, &mut world) {}

        let counter = world.get::<Counter>(entity).unwrap();

        assert_eq!(counter.0, 3);
    }

    #[test]
    fn test_retry_while() {
        let mut world = World::default();

        #[derive(Component)]
        struct Counter(u32);

        let mut retry_system = IntoSystem::into_system(
            move |In(entity): In<Entity>, mut counters: Query<&mut Counter>| {
                let mut counter = counters.get_mut(entity).unwrap();

                counter.0 += 1;

                if counter.0 < 10 {
                    Status::Failure
                } else {
                    Status::Success
                }
            },
        )
        .retry_while(|In(entity): In<Entity>, counters: Query<&Counter>| {
            let counter = counters.get(entity).unwrap();
            counter.0 < 5
        });

        retry_system.initialize(&mut world);

        let entity = world.spawn(Counter(0)).id();

        let mut last_status: Status = Status::Running;

        for _ in 0..=10 {
            last_status = retry_system.run(entity, &mut world);

            if matches!(last_status, Status::Failure) {
                break;
            }
        }

        let counter = world.get::<Counter>(entity).unwrap();

        assert_eq!(last_status, Status::Failure);
        assert_eq!(counter.0, 5);
    }

    #[test]
    fn test_chain() {
        let mut world = World::default();

        let mut chained = Compositor::chain((
            IntoSystem::into_system(fail),
            IntoSystem::into_system(panic_if_run),
        ));

        chained.initialize(&mut world);

        let entity = world.spawn_empty().id();

        assert_eq!(chained.run(entity, &mut world), Status::Failure);
    }

    #[test]
    fn test_select() {
        let mut world = World::default();

        #[derive(Component)]
        struct HasRun(bool);

        let mut selected = Compositor::select((
            IntoSystem::into_system(fail),
            IntoSystem::into_system(fail),
            IntoSystem::into_system(|In(entity): In<Entity>, mut has_run: Query<&mut HasRun>| {
                has_run.get_mut(entity).unwrap().0 = true;
                Status::Success
            }),
            IntoSystem::into_system(panic_if_run),
        ));

        selected.initialize(&mut world);

        let entity = world.spawn(HasRun(false)).id();

        let mut last_status: Status = Status::Running;

        for _ in 0..=1 {
            last_status = selected.run(entity, &mut world);
            if let Status::Success = last_status {
                break;
            }
        }

        assert_eq!(
            last_status,
            Status::Running,
            "select did not indicate running"
        );

        for _ in 0..=1 {
            last_status = selected.run(entity, &mut world);
            if let Status::Success = last_status {
                break;
            }
        }

        assert_eq!(last_status, Status::Success, "select did not short circuit");

        let has_run = world.query::<&HasRun>().get(&world, entity).unwrap();
        assert!(has_run.0, "select system did not run");
    }
}
