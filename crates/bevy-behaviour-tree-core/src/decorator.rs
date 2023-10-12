use bevy::prelude::{Entity, IntoSystem, System, World};

use crate::{
    prelude::{Behaviour, Status},
    TodoBehaviour,
};

/// Types that can be used with the built-in decorator functions.
/// - [`Behaviour`]
/// - Nothing else lol
pub trait Decorator {
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

impl<T: Behaviour> Decorator for T {
    fn invert(self) -> impl Behaviour {
        Invert(self)
    }

    fn retry(self, tries: usize) -> impl Behaviour {
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