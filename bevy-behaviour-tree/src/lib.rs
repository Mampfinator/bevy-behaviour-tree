//! bevy-behaviour-tree is a crate for defining simple, composable, and extensible behaviour trees for [bevy].
#![warn(missing_docs)]
#![feature(return_position_impl_trait_in_trait)]

use bevy::{
    prelude::{Entity, IntoSystem, System, World},
    utils::all_tuples,
};

/// The trait at the core of this crate.
///
/// The idea is simple: a `Behaviour` takes in an [`Entity`] and the [`World`] it belongs to, along with its own arbitrary state, and returns a [`Status`], indicating whether it's running, has failed or succeeded.
///
/// The most important implementation for `Behaviour` is a blanket implementation for any [`System<In = Entity, Out = Status>`][bevy::ecs::system::System],
/// meaning that any user-defined system that takes in an `Entity` and returns a `Status` is automatically a `Behaviour`.
/// If you've never seen or used system inputs before, have a look at [`In`] and the [piping example](https://github.com/bevyengine/bevy/blob/main/examples/ecs/system_piping.rs).
///
/// There are three basic types of behaviours:
///  - *Leafs*: they access and/or modify world state directly. These are usually user defined, like a system to make an entity walk from A to B, or to check if there are enemies nearby.
///  - *Decorators*: they modify the output of another behaviour, like [`invert`][DecoratorInput::invert] and [`retry_while`][DecoratorInput::retry_while].
///  - *Compositors*: they modify the output of a group of other behaviours, like [`select`] and [`chain`]
///
/// These types aren't strictly enforced, but are the defacto standard implementation for behaviour tree nodes. You can freely extend and mix them as you see fit, by using the aforementioned system piping for example.
/// As long as the resulting system takes in an `Entity` and outputs a `Status`, it's a valid `Behaviour` usable with this crate.
///
/// For more complex custom implementations, you need to make sure that all underlying systems [initialize][bevy::ecs::system::System::initialize] correcly.
/// If you see a `Encountered a mismathed World.` panic, it's likely one of the systems you rely on wasn't initialized properly.
pub trait Behaviour: Send + Sync + 'static {
    /// Runs the behaviour on the given entity, in the given world. Called once a world tick at most.
    ///
    /// # Panics
    /// If the world passed is not the same one that was passed in [`initialize`][Behaviour::initialize].
    fn run(&mut self, entity: Entity, world: &mut World) -> Status;

    /// Initializes the behaviour. This registers component access for underlying systems, and does general setup work.
    /// Required to be called before [`run`][Behaviour::run].
    fn initialize(&mut self, world: &mut World);
}

/// The status of a [`Behaviour`], returned when it's [`run`][Behaviour::run].
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Status {
    /// Indicates a successful action.
    Success,
    /// Indicates a failed action.
    Failure,
    /// Indicates that an action requires more time to complete.
    Running,
}

impl<F: System<In = Entity, Out = Status> + Clone> Behaviour for F {
    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        System::run(self, entity, world)
    }

    fn initialize(&mut self, world: &mut World) {
        System::initialize(self, world)
    }
}

/// For debug purposes only. Panics if used in any way.
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct TodoBehaviour;

impl Behaviour for TodoBehaviour {
    fn initialize(&mut self, _: &mut World) {
        todo!()
    }

    fn run(&mut self, _: Entity, _: &mut World) -> Status {
        todo!()
    }
}

/// Types that can be used with the built-in decorator functions.
/// - [`Behaviour`]
/// - Nothing else lol
trait DecoratorInput {
    /// Inverts the output.
    ///
    /// **Succeeds** when the underlying behaviour fails.
    /// **Fails** when the underlying behaviour succeeds.
    fn invert(self) -> impl Behaviour;

    /// Retry the action a fixed number of times.
    ///
    /// **Succeeds** when the underlying behaviour succeeds.
    /// **Fails** when the maximum amount of retries has been reached.
    fn retry(self, tries: usize) -> impl Behaviour;

    /// Retries while the condition is true.
    ///
    /// **Succeeds** when the underlying behaviour succeeds.
    /// **Fails** when the condition becomes false.
    fn retry_while<Marker, C>(self, condition: C) -> impl Behaviour
    where
        C: IntoSystem<Entity, bool, Marker> + Clone,
        <C as IntoSystem<Entity, bool, Marker>>::System: Clone;

    /// Repeat a fixed number of times, regardless of whether or not the underlying behaviour fails or not.
    ///
    /// **Succeeds** after running `repeats` times.
    fn repeat(self, repeats: usize) -> impl Behaviour;

    /// Repeat while the condition is true, regardless of whether or not the underlying behaviour fails or not.
    ///
    /// **Succeeds** after the condition becomes true.
    fn repeat_while<C>(self, condition: C) -> impl Behaviour
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone;
}

/// See [`DecoratorInput::invert`].
#[derive(Clone)]
struct Invert<T: Behaviour>(T);

impl<T: Behaviour> Behaviour for Invert<T> {
    fn initialize(&mut self, world: &mut World) {
        self.0.initialize(world);
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        match self.0.run(entity, world) {
            Status::Failure => Status::Success,
            Status::Success => Status::Failure,
            Status::Running => Status::Running,
        }
    }
}

/// See [`DecoratorInput::retry_while`].
#[derive(Clone)]
struct RetryWhile<F: Behaviour, C: System<In = Entity, Out = bool> + Clone> {
    func: F,
    condition: C,
}

impl<F: Behaviour, C: System<In = Entity, Out = bool> + Clone> Behaviour for RetryWhile<F, C> {
    fn initialize(&mut self, world: &mut World) {
        self.condition.initialize(world);
        self.func.initialize(world);
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        if self.condition.run(entity, world) {
            match self.func.run(entity, world) {
                Status::Failure | Status::Running => Status::Running,
                Status::Success => Status::Success,
            }
        } else {
            Status::Failure
        }
    }
}

/// See [`DecoratorInput::retry`].
#[derive(Clone)]
struct Retry<T: Behaviour> {
    max_tries: usize,
    tries: usize,
    func: T,
}

impl<T: Behaviour> Behaviour for Retry<T> {
    fn initialize(&mut self, world: &mut World) {
        self.func.initialize(world);
        self.tries = 0;
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        match self.func.run(entity, world) {
            Status::Failure => {
                self.tries += 1;
                if self.tries < self.max_tries {
                    Status::Running
                } else {
                    self.tries = 0; // reset state to get ready for the next call
                    Status::Failure
                }
            }
            Status::Success => {
                self.tries = 0; // reset state
                Status::Success
            }
            Status::Running => Status::Running,
        }
    }
}

impl<T: Behaviour> DecoratorInput for T {
    fn invert(self) -> Invert<T> {
        Invert(self)
    }

    fn retry(self, tries: usize) -> Retry<T> {
        Retry {
            func: self,
            max_tries: tries,
            tries: 0,
        }
    }

    fn retry_while<Marker, C>(self, condition: C) -> impl Behaviour
    where
        C: IntoSystem<Entity, bool, Marker> + Clone,
        <C as IntoSystem<Entity, bool, Marker>>::System: Clone,
    {
        RetryWhile {
            func: self,
            condition: IntoSystem::into_system(condition),
        }
    }

    fn repeat(self, _times: usize) -> impl Behaviour {
        TodoBehaviour
    }

    fn repeat_while<C>(self, _condition: C) -> impl Behaviour
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone,
    {
        TodoBehaviour
    }
}

/// Helper trait for [`Behaviour`] tuples.
trait BehaviourGroup {
    fn group(self) -> Vec<Box<dyn Behaviour>>;
}

macro_rules! impl_behaviour_group {
    ($($name: ident), *) => {
        impl<$($name: Behaviour),*> BehaviourGroup for ($($name,)*) {
            fn group(self) -> Vec<Box<dyn Behaviour>> {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;

                vec![$(Box::new($name)),*]
            }
        }
    }
}

all_tuples!(impl_behaviour_group, 2, 15, B);

/// *Composite* nodes take a group of input nodes, run them and transform their ouput.
pub trait CompositeInput {
    /// Chains the input nodes together. This breaks with a failure as soon as one of the input nodes fails.
    fn chain(self) -> impl Behaviour;
    /// Selects between the input branches. Breaks with a success as soon as one input succeeds, or fails if none of them do.
    fn select(self) -> impl Behaviour;
}

impl<T: BehaviourGroup> CompositeInput for T {
    fn chain(self) -> impl Behaviour {
        Chain {
            funcs: BehaviourGroup::group(self),
            index: 0,
        }
    }

    fn select(self) -> impl Behaviour {
        Select {
            funcs: BehaviourGroup::group(self),
            index: 0,
        }
    }
}

/// See [`CompositeInput::chain`].
struct Chain {
    funcs: Vec<Box<dyn Behaviour>>,
    index: usize,
}

impl Behaviour for Chain {
    fn initialize(&mut self, world: &mut World) {
        for func in &mut self.funcs {
            func.initialize(world);
        }
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        if let Some(system) = self.funcs.get_mut(self.index) {
            match system.run(entity, world) {
                Status::Running => Status::Running,
                Status::Failure => {
                    // reset state
                    self.index = 0;
                    Status::Failure
                }
                Status::Success => {
                    self.index += 1;
                    Status::Running
                }
            }
        } else {
            Status::Success
        }
    }
}

/// See [`CompositeInput::select`].
struct Select {
    funcs: Vec<Box<dyn Behaviour>>,
    index: usize,
}

impl Behaviour for Select {
    fn initialize(&mut self, world: &mut World) {
        for func in &mut self.funcs {
            func.initialize(world);
        }
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        if let Some(system) = self.funcs.get_mut(self.index) {
            match system.run(entity, world) {
                Status::Running => Status::Running,
                Status::Failure => {
                    // advance to the next branch
                    self.index += 1;
                    Status::Running
                }
                Status::Success => {
                    // reset state
                    self.index = 0;
                    Status::Success
                }
            }
        } else {
            self.index = 0;
            // we tried everything; no branch was successful
            Status::Failure
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Component, Entity, In, IntoSystem, Query, World};

    use crate::{Behaviour, CompositeInput, DecoratorInput, Status};

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

        let mut chained = CompositeInput::chain((
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

        let mut selected = CompositeInput::select((
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
