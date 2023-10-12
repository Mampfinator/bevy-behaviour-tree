use bevy::{
    prelude::{Entity, World},
    utils::{all_tuples, HashMap},
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
    /// Runs the input nodes sequentially.
    /// 
    /// **Succeeds** if all input nodes succeed.
    /// **Fails** if any input node fails.
    fn sequence(self) -> Sequence;
    /// Selects between the input branches.
    /// 
    /// **Succeeds** as soon as any node succeeds. **Fails** if all of them fail.
    fn select(self) -> Select;
}

impl<Marker, T: BehaviourGroup<Marker>> Compositor<Marker> for T {
    fn sequence(self) -> Sequence {
        Sequence {
            funcs: BehaviourGroup::group(self),
            indices: HashMap::default(),
        }
    }

    fn select(self) -> Select {
        Select {
            funcs: BehaviourGroup::group(self),
            indices: HashMap::default()
        }
    }
}

/// See [`Compositor::chain`].
pub struct Sequence {
    funcs: Vec<Box<dyn Behaviour>>,
    indices: HashMap<Entity, usize>,
}

impl Sequence {
    fn index(&mut self, entity: Entity) -> usize {
        match self.indices.get(&entity) {
            Some(index) => *index,
            None => {
                self.indices.insert(entity, 0);
                0
            }
        }
    }

    fn reset(&mut self, entity: Entity) {
        self.indices.insert(entity, 0);
    }

    fn increase(&mut self, entity: Entity) {
        if let Some(index) = self.indices.get_mut(&entity) {
            *index += 1;
        }
    }

    pub(crate) fn behaviour_mut(&mut self, entity: Entity) -> Option<&mut Box<dyn Behaviour>> {
        let index = self.index(entity);
        self.funcs.get_mut(index)
    }
}

impl IntoBehaviour<SelfMarker> for Sequence {
    fn into_behaviour(self) -> impl Behaviour {
        self
    }
}

impl Behaviour for Sequence {
    fn initialize(&mut self, world: &mut World) {
        for func in &mut self.funcs {
            func.initialize(world);
        }
    }

    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        if let Some(behaviour) = self.behaviour_mut(entity) {
            match behaviour.run(entity, world) {
                Status::Running => Status::Running,
                Status::Failure => {
                    self.reset(entity);
                    Status::Failure
                },
                Status::Success => {
                    self.increase(entity);
                    Status::Running
                }
            }
        } else {
            self.reset(entity);
            Status::Success
        }
    }
}

/// See [`CompositeInput::select`].
pub struct Select {
    funcs: Vec<Box<dyn Behaviour>>,
    indices: HashMap<Entity, usize>,
}

impl Select {
    pub(crate) fn behaviour_mut(&mut self, entity: Entity) -> Option<&mut Box<dyn Behaviour>> {
        let index = self.index(entity);
        self.funcs.get_mut(index)
    }

    fn index(&mut self, entity: Entity) -> usize {
        match self.indices.get(&entity) {
            Some(index) => *index,
            None => {
                self.indices.insert(entity, 0);
                0
            }
        }
    }

    fn reset(&mut self, entity: Entity) {
        if let Some(index) = self.indices.get_mut(&entity) {
            *index = 0;
        }
    }

    pub(crate) fn increase(&mut self, entity: Entity) {
        if let Some(index) = self.indices.get_mut(&entity) {
            *index += 1;
        }
    }
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
        if let Some(behaviour) = self.behaviour_mut(entity) {
            match behaviour.run(entity, world) {
                Status::Running => Status::Running,
                Status::Failure => {
                    self.increase(entity);
                    Status::Running
                },
                Status::Success => {
                    self.reset(entity);
                    Status::Success
                }
            }
        } else {
            self.reset(entity);
            // we tried everything; no branch was successful
            Status::Failure
        }
    }
}
