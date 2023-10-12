use bevy::{
    prelude::{Entity, World},
    utils::all_tuples,
};

use crate::{
    behaviour::{IntoBehaviour, SelfMarker},
    prelude::{Behaviour, Status},
};

/// Helper trait for [`Behaviour`] tuples.
trait BehaviourGroup<Marker> {
    fn group(self) -> Vec<Box<dyn Behaviour>>;
}

macro_rules! impl_behaviour_group {
    ($(($name:ident,$marker:ident)),*) => {
        impl<$($marker: 'static, $name: IntoBehaviour<$marker>),*> BehaviourGroup<($($marker,)*)> for ($($name,)*) {
            fn group(self) -> Vec<Box<dyn Behaviour>> {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;

                vec![$(Box::new(IntoBehaviour::into_behaviour($name))),*]
            }
        }
    }
}

all_tuples!(impl_behaviour_group, 2, 15, B, M);

/// *Composite* nodes take a group of input nodes, run them and transform their ouput.
pub trait Compositor<Marker> {
    /// Chains the input nodes together. This breaks with a failure as soon as one of the input nodes fails.
    fn chain(self) -> Chain;
    /// Selects between the input branches. Breaks with a success as soon as one input succeeds, or fails if none of them do.
    fn select(self) -> Select;
}

impl<Marker, T: BehaviourGroup<Marker>> Compositor<Marker> for T {
    fn chain(self) -> Chain {
        Chain {
            funcs: BehaviourGroup::group(self),
            index: 0,
        }
    }

    fn select(self) -> Select {
        Select {
            funcs: BehaviourGroup::group(self),
            index: 0,
        }
    }
}

/// See [`CompositeInput::chain`].
pub struct Chain {
    funcs: Vec<Box<dyn Behaviour>>,
    index: usize,
}

impl IntoBehaviour<SelfMarker> for Chain {
    fn into_behaviour(self) -> impl Behaviour {
        self
    }
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
pub struct Select {
    funcs: Vec<Box<dyn Behaviour>>,
    index: usize,
}

impl IntoBehaviour<SelfMarker> for Select {
    fn into_behaviour(self) -> impl Behaviour {
        self
    }
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
