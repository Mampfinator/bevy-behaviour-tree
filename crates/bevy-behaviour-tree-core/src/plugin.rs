use bevy::{
    ecs::schedule::ScheduleLabel,
    prelude::{
        App, Component, Entity, Mut, Plugin, ReflectComponent, Resource, Update, Without, World,
    },
    reflect::Reflect,
    utils::HashSet,
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
    // We Option<T> here so we can temporarily move behaviours out of the resource without shifting indices with `mem::take`.
    trees: Vec<Option<Box<dyn Behaviour>>>,
    initialized: HashSet<BehaviourId>,
}

impl BehaviourTrees {
    /// Add a new behaviour to the tree.
    ///
    /// [`Behaviour`]s are a basically just systems - they have full access to the world, but they always take in an [`Entity`] and return a [`Status`][super::prelude::Status].
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use crate::prelude::*;
    ///
    /// fn rotate(In(entity), query: Query<&mut Transform>) -> Status {
    ///     let Ok(mut transform) = query.get_mut(entity) else {
    ///         return Status::Failure
    ///     };
    ///
    ///     transform.rotate(Quat::from_axis_rotation(transform.local_z), 90.0_f32.to_radians());
    ///
    ///     Status::Running
    /// }
    ///
    /// fn system(
    ///     mut trees: ResMut<BehaviourTrees>,
    ///     mut commands: Commands,
    /// ) {
    ///     let behaviour_id = trees.create(rotate.repeat_forever());
    ///     commands.spawn((TransformBundle::default(), behaviour_id));
    /// }
    ///
    /// # bevy::ecs::system::assert_is_system(system);
    /// ```
    pub fn create(&mut self, behaviour: impl Behaviour + 'static) -> BehaviourId {
        self.trees.push(Some(Box::new(behaviour)));
        BehaviourId(self.trees.len() - 1)
    }

    /// Temporarily moves the behaviour belonging to `id` out of the internal storage.
    /// Used for behaviour initialization logic.
    ///
    /// `scope` is not ran if the behaviour doesn't exist.
    pub(crate) fn behaviour_scope<F>(&mut self, id: BehaviourId, mut scope: F)
    where
        F: FnMut(&mut Self, &mut Box<dyn Behaviour>),
    {
        let Some(behaviour_borrow) = self.trees.get_mut(id.0) else {
            return;
        };

        let mut behaviour = std::mem::take(behaviour_borrow).unwrap();

        scope(self, &mut behaviour);

        self.trees[id.0] = Some(behaviour);
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
        let mut query = world
            .query_filtered::<(Entity, &BehaviourId), Without<Skip>>()
            .iter(world)
            .map(|(entity, id)| (entity, *id))
            .collect::<Vec<_>>(); // collect so we can reborrow world for initialization/running.

        // sort to *hopefully* squeeze out some performance.
        query.sort_by(|(_, id1), (_, id2)| id1.cmp(id2));

        for (entity, id) in query {
            trees.behaviour_scope(id, |trees, behaviour| {
                if !trees.initialized.contains(&id) {
                    behaviour.initialize(world);
                    trees.initialized.insert(id);
                }

                behaviour.run(entity, world);
            });
        }
    });
}
