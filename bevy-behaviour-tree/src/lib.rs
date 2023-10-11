#![feature(return_position_impl_trait_in_trait)]

use bevy::{prelude::{System, Entity, World, In, IntoSystemConfigs, IntoSystemSetConfigs, IntoSystem, GlobalTransform}, ecs::{schedule::{SystemConfig, SystemConfigs}, system::{BoxedSystem, FunctionSystem, ExclusiveFunctionSystem}}};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Status {
    Success,
    Failure,
    Running,
}

/// The trait at the core of this crate.
/// 
/// The idea is simple: a `Behaviour` takes in an [`Entity`] and the [`World`] it belongs to, along with its own arbitrary state, and returns a [`Status`], indicating whether it's running, has failed or succeeded.
/// 
/// The most important implementation for `Behaviour` is a blanket implementation for any [`System<In = Entity, Out = Status>`][bevy::ecs::system::System], 
/// meaning that any user-defined system that takes in an `Entity` and returns a `Status` is automatically a `Behaviour`.
/// See [`In`] if you've never used system input before.
/// 
/// There are three basic types of behaviours:
///  - *Actions*: they modify world state directly. These are usually user defined, like a system to make an entity walk from A to B.
///  - *Decorators*: they modify the output of another behaviour, like [`invert`][DecoratorInput::invert] and [`retry_while`][DecoratorInput::retry_while].
///  - *Compositors*: they modify the output of a group of other behaviours, like [`select`] and [`chain`]
/// 
/// These types aren't strictly enforced, but are the defacto standard implementation for behaviour tree nodes. You can freely extend and mix them as you see fit, by using bevy's [`pipe systems`][bevy::ecs::system::Pipe] for example.
/// As long as the resulting system takes in an `Entity` and outputs a `Status`, it's a valid `Behaviour` usable with this crate.
pub trait Behaviour: Send + Sync + 'static + Clone {
    fn run(&mut self, entity: Entity, world: &mut World) -> Status;
    fn initialize(&mut self, world: &mut World);
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

trait DecoratorInput {
    /// Inverts the output of this behaviour group.
    /// When the group succeeds, this fails. When the group fails, this succeeds.
    fn invert(self) -> impl Behaviour;

    /// Retry the action a fixed number of times.
    fn retry(self, tries: usize) -> impl Behaviour;

    /// Retries while the condition is true, fails when it becomes false.
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_behaviour_tree::*;
    /// fn system() {
    /// 
    /// }
    /// ```
    fn retry_while<Marker, C>(self, condition: C) -> impl Behaviour
        where 
            C: IntoSystem<(), bool, Marker> + Clone,
            <C as IntoSystem<(), bool, Marker>>::System: Clone;

    fn repeat(self, times: usize) -> impl Behaviour;
    fn repeat_while<C>(self, condition: C) -> impl Behaviour
        where
            C: IntoSystem<(), bool, ()> + Clone,
            <C as IntoSystem<(), bool, ()>>::System: Clone;
}

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

#[derive(Clone)]
struct RetryWhile<F: Behaviour, C: System<In = (), Out = bool> + Clone> {
    func: F,
    condition: C,
}

impl<F: Behaviour, C: System<In = (), Out = bool> + Clone> Behaviour for RetryWhile<F, C> {
    fn initialize(&mut self, world: &mut World) {
        self.condition.initialize(world);
        self.func.initialize(world);
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        if self.condition.run((), world) {
            match self.func.run(entity, world) {
                Status::Failure | Status::Running => Status::Running,
                Status::Success => Status::Success
            }
        } else {
            Status::Failure
        }
    }
}
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
            },
            Status::Success => {
                self.tries = 0; // reset state
                Status::Success
            },
            Status::Running => {
                Status::Running
            }
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
            C: IntoSystem<(), bool, Marker> + Clone,
            <C as IntoSystem<(), bool, Marker>>::System: Clone {
        RetryWhile {
            func: self,
            condition: IntoSystem::into_system(condition),
        }
    }

    fn repeat(self, times: usize) -> impl Behaviour {
        TodoBehaviour
    }

    fn repeat_while<C>(self, condition: C) -> impl Behaviour
        where
            C: IntoSystem<(), bool, ()> + Clone,
            <C as IntoSystem<(), bool, ()>>::System: Clone {
        TodoBehaviour
    }
}

pub struct CompositeInput {
    systems: Vec<Box<dyn System<In = Entity, Out = Status>>>,
}

impl CompositeInput {
    /// Run the underlying system. Returns `None` if no system with the specified index exists.
    pub(crate) fn run(&mut self, index: usize, entity: Entity, world: &mut World) -> Option<Status> {
        let system = self.systems.get_mut(index)?;
        Some(system.run(entity, world))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::{AtomicI32, Ordering}, Arc};

    use bevy::prelude::{In, Entity, World, IntoSystem};

    use crate::{Status, DecoratorInput, Behaviour};

    fn succeed(In(_): In<Entity>) -> Status {
        Status::Success
    }

    #[test]
    fn test_invert() {
        let mut world = World::default();

        let mut system = IntoSystem::into_system(succeed)
            .invert();

        let entity = world.spawn_empty().id();

        Behaviour::initialize(&mut system, &mut world);

        let status = Behaviour::run(&mut system, entity, &mut world);

        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_retry() {
        let mut world = World::default();
        let counter = Arc::new(AtomicI32::new(0));

        let i = Arc::clone(&counter);

        let system = IntoSystem::into_system(move |_: In<Entity>| {            
            i.fetch_add(1, Ordering::Relaxed);

            if i.load(Ordering::Relaxed) < 3 {
                Status::Failure
            } else {
                Status::Success
            }
        });

        let mut retry = system.retry(5);

        retry.initialize(&mut world);

        let entity = world.spawn_empty().id();

        while let Status::Running = retry.run(entity, &mut world) {
            
        }

        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }
}
