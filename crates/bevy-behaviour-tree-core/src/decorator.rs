use bevy::prelude::{Entity, IntoSystem, System, World};

use crate::{
    behaviour::{IntoBehaviour, SelfMarker},
    prelude::{Behaviour, Status},
    TodoBehaviour,
};

/// Types that can be used with the built-in decorator functions.
/// - [`Behaviour`]
/// - Nothing else lol
pub trait Decorator<Marker> {
    /// Inverts the output.
    ///
    /// **Succeeds** when the underlying behaviour fails.
    /// **Fails** when the underlying behaviour succeeds.
    fn invert(self) -> impl Behaviour + IntoBehaviour<SelfMarker>;

    /// Only runs the underlying behaviour if the condition returns true.
    ///
    /// **Succeeds** if the condition is false and short circuits.
    /// **Succeeds or fails** depending on the underlying behaviour if the condition is true.
    fn run_if<C>(self, condition: C) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone;

    /// Like [`run_if`][Decorator::run_if], but with a customisable return when short circuiting.
    ///
    /// Note that `system.run_if(|| true).invert()` is equivalent to `system.run_if_with_return(|| true, Status::Failure)`.
    fn run_if_with_return<C>(
        self,
        condition: C,
        short_circuit: Status,
    ) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone;

    /// Retry the action a fixed number of times.
    ///
    /// **Succeeds** when the underlying behaviour succeeds.
    /// **Fails** when the maximum amount of retries has been reached.
    fn retry(self, tries: usize) -> impl Behaviour + IntoBehaviour<SelfMarker>;

    /// Retries while the condition is true.
    ///
    /// **Succeeds** when the underlying behaviour succeeds.
    /// **Fails** when the condition becomes false.
    fn retry_while<CMarker, C>(self, condition: C) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, CMarker> + Clone,
        <C as IntoSystem<Entity, bool, CMarker>>::System: Clone;

    /// Repeat a fixed number of times, regardless of whether or not the underlying behaviour fails or not.
    ///
    /// **Succeeds** after running `repeats` times.
    fn repeat(self, repeats: usize) -> impl Behaviour + IntoBehaviour<SelfMarker>;

    /// Repeat while the condition is true, regardless of whether or not the underlying behaviour fails or not.
    ///
    /// **Succeeds** after the condition becomes false.
    fn repeat_while<C>(self, condition: C) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone;
}

impl<Marker: 'static, T: IntoBehaviour<Marker>> Decorator<Marker> for T {
    fn invert(self) -> impl Behaviour + IntoBehaviour<SelfMarker> {
        Invert(IntoBehaviour::into_behaviour(self))
    }

    fn run_if<C>(self, condition: C) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone,
    {
        self.run_if_with_return(condition, Status::Success)
    }

    fn run_if_with_return<C>(
        self,
        condition: C,
        short_circuit: Status,
    ) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone,
    {
        RunIf {
            func: IntoBehaviour::into_behaviour(self),
            condition: IntoSystem::into_system(condition),
            short_circuit,
        }
    }

    fn retry(self, tries: usize) -> impl Behaviour + IntoBehaviour<SelfMarker> {
        Retry {
            func: IntoBehaviour::into_behaviour(self),
            max_tries: tries,
            tries: 0,
        }
    }

    fn retry_while<CMarker, C>(self, condition: C) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, CMarker> + Clone,
        <C as IntoSystem<Entity, bool, CMarker>>::System: Clone,
    {
        RetryWhile {
            func: IntoBehaviour::into_behaviour(self),
            condition: IntoSystem::into_system(condition),
        }
    }

    fn repeat(self, _times: usize) -> impl Behaviour + IntoBehaviour<SelfMarker> {
        TodoBehaviour
    }

    fn repeat_while<C>(self, _condition: C) -> impl Behaviour + IntoBehaviour<SelfMarker>
    where
        C: IntoSystem<Entity, bool, ()> + Clone,
        <C as IntoSystem<Entity, bool, ()>>::System: Clone,
    {
        TodoBehaviour
    }
}

/// See [`DecoratorInput::invert`].
#[derive(Clone)]
struct Invert<T: Behaviour>(T);

impl<T: Behaviour> IntoBehaviour<SelfMarker> for Invert<T> {
    fn into_behaviour(self) -> impl Behaviour {
        self
    }
}

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

struct RunIf<F: Behaviour, C: System<In = Entity, Out = bool> + Clone> {
    func: F,
    condition: C,
    short_circuit: Status,
}

impl<F: Behaviour, C: System<In = Entity, Out = bool> + Clone> IntoBehaviour<SelfMarker>
    for RunIf<F, C>
{
    fn into_behaviour(self) -> impl Behaviour {
        self
    }
}

impl<F: Behaviour, C: System<In = Entity, Out = bool> + Clone> Behaviour for RunIf<F, C> {
    fn initialize(&mut self, world: &mut World) {
        self.func.initialize(world);
        self.condition.initialize(world);
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        if self.condition.run(entity, world) {
            self.func.run(entity, world)
        } else {
            self.short_circuit
        }
    }
}

/// See [`DecoratorInput::retry_while`].
#[derive(Clone)]
struct RetryWhile<F: Behaviour, C: System<In = Entity, Out = bool> + Clone> {
    func: F,
    condition: C,
}

impl<F: Behaviour, C: System<In = Entity, Out = bool> + Clone> IntoBehaviour<SelfMarker>
    for RetryWhile<F, C>
{
    fn into_behaviour(self) -> impl Behaviour {
        self
    }
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

impl<T: Behaviour> IntoBehaviour<SelfMarker> for Retry<T> {
    fn into_behaviour(self) -> impl Behaviour {
        self
    }
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
