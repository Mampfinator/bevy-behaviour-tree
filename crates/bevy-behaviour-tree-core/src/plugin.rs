use bevy::{
    ecs::schedule::ScheduleLabel,
    prelude::{
        App, Component, Entity, Mut, Plugin, ReflectComponent, Resource, Update, Without, World,
    },
    reflect::Reflect,
};

use crate::prelude::Behaviour;

/// Plugin for all core functionality.
pub struct BehaviourTreePlugin<Label: ScheduleLabel + Clone = Update> {
    label: Label,
}

impl Default for BehaviourTreePlugin {
    fn default() -> Self {
        Self { label: Update }
    }
}

impl Plugin for BehaviourTreePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BehaviourTrees>()
            .add_systems(self.label.clone(), run_ticks);
    }
}

/// Resource required for creating trees.
#[derive(Resource, Default)]
pub struct BehaviourTrees {
    trees: Vec<Box<dyn Behaviour>>,
}

impl BehaviourTrees {
    /// Add a new behaviour tree and return its ID.
    /// THIS API IS A WORK IN PROGRESS. It's going to be much less verbose once it's stable.
    pub fn add(&mut self, behaviour: impl Behaviour + 'static) -> BehaviourId {
        self.trees.push(Box::new(behaviour));
        BehaviourId(self.trees.len() - 1)
    }
}

/// Skips processing the behaviour tree for this entity.
#[derive(Component, PartialEq, Eq, Debug, Default)]
pub struct Skip;

/// An ID for a behaviour tree.
/// This is a component type. If this is on an entity, that entity is ticked for the given tree.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct BehaviourId(usize);

fn run_ticks(world: &mut World) {
    world.resource_scope(|world: &mut World, mut trees: Mut<BehaviourTrees>| {
        let query = world
            .query_filtered::<(Entity, &BehaviourId), Without<Skip>>()
            .iter(world)
            .map(|(entity, id)| (entity, *id))
            .collect::<Vec<_>>(); // collect so we can reborrow world.

        for (entity, id) in query {
            if let Some(behaviour) = trees.trees.get_mut(id.0) {
                behaviour.run(entity, world);
            } else {
                bevy::log::warn!(
                    "Trying to run a behaviour tree that doesn't exist: {}",
                    id.0
                );
            }
        }
    });
}
