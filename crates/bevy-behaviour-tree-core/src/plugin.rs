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

impl<Label: ScheduleLabel + Clone> BehaviourTreePlugin<Label> {
    /// Executes the tree runner in the given schedule.
    /// Defaults to [`Update`].
    pub fn in_schedule(label: Label) -> Self {
        Self { label }
    }
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
    // We use Option<T> here so we can temporarily move behaviours out of the resource without shifting indices with `std::mem::take`.
    trees: Vec<Option<Box<dyn Behaviour>>>,
    initialized: HashSet<BehaviourId>,
}

impl BehaviourTrees {
    /// Create a new behaviour tree.
    ///
    /// Behaviour trees are evaluated every tick. If you want to only run a tree under certain conditions,
    /// you can just add a top-level [`run_if`][`crate::decorator::Decorator::run_if`].
    ///
    /// [`Behaviour`]s are a basically just systems - they have full access to the world, but they always take in an [`Entity`] and return a [`Status`][super::prelude::Status].
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_behaviour_tree_core::prelude::*;
    ///
    /// fn rotate(In(entity): In<Entity>, mut query: Query<&mut Transform>) -> Status {
    ///     let Ok(mut transform) = query.get_mut(entity) else {
    ///         return Status::Failure;
    ///     };
    ///     
    ///     let axis = transform.local_z();
    ///     transform.rotate(Quat::from_axis_angle(axis, 90.0_f32.to_radians()));
    ///
    ///     Status::Running
    /// }
    ///
    /// fn system(
    ///     mut trees: ResMut<BehaviourTrees>,
    ///     mut commands: Commands,
    /// ) {
    ///     let behaviour_id = trees.create(rotate.repeat(50));
    ///     commands.spawn((TransformBundle::default(), behaviour_id));
    /// }
    ///
    /// # bevy::ecs::system::assert_is_system(system);
    /// ```
    ///
    /// `Behaviours` can also return `Option<Status>`, where `None` indicates failure.
    /// This is useful if the [`Behaviour`] needs to access query items and should fail if the query doesn't contain said items.
    /// We can rewrite the above `rotate` behaviour like so:
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_behaviour_tree_core::prelude::*;
    ///
    /// fn rotate(In(entity): In<Entity>, mut query: Query<&mut Transform>) -> Option<Status> {
    ///     let mut transform = query.get_mut(entity).ok()?;
    ///
    ///     let axis = transform.local_z();
    ///     transform.rotate(Quat::from_axis_angle(axis, 90.0_f32.to_radians()));
    ///
    ///     Some(Status::Running)
    /// }
    /// # fn system(
    /// #     mut trees: ResMut<BehaviourTrees>,
    /// #     mut commands: Commands,
    /// # ) {
    /// #     let behaviour_id = trees.create(rotate.repeat(50));
    /// #     commands.spawn((TransformBundle::default(), behaviour_id));
    /// # }
    ///
    /// # bevy::ecs::system::assert_is_system(system);
    ///
    /// ```
    /// You can return any [`Into<Status>`] from a behaviour, by the way! By default, this is only implemented for `Option<Status>` and `bool` (and, y'know, `Status` itself).
    pub fn create<T: Behaviour + 'static>(&mut self, behaviour: T) -> BehaviourId {
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
