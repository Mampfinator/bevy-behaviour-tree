//! Showcases how to construct and use basic behaviour trees by spawning 100 agents that randomly walk across the screen.
use bevy::{prelude::*, math::Vec3Swizzles};
use bevy_behaviour_tree::prelude::*;
use rand::{thread_rng, Rng};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BehaviourTreePlugin::default()))
        .add_systems(Startup, (spawn_camera, spawn_agents))
        .add_systems(Update, visualize)
        .run();
}

fn spawn_camera(
    mut commands: Commands,
) {
    commands.spawn(Camera2dBundle::default());
}

/// A marker component so we avoid moving the camera by accident!
#[derive(Component)]
struct Agent;

fn spawn_agents(
    mut commands: Commands,
    mut trees: ResMut<BehaviourTrees>,
    window: Query<&Window>,
) {
    let behaviour = trees.create((
        (has_target, pick_target).select(), // read as "if we have a target, continue. If not, pick one, then continue."
        walk_to_target,
    ).sequence());

    let window = window.single();
    let x = window.width();
    let y = window.height();

    let window_size = Vec2::new(x, y);

    for _ in 0..100 {
        commands.spawn((Agent, random_transform(window_size), behaviour));
    }
}

fn random_transform(window_size: Vec2) -> TransformBundle {
    TransformBundle::from_transform(Transform::from_translation(random_point(window_size).extend(0.)))
}

fn random_point(window_size: Vec2) -> Vec2 {
    let mut rng = thread_rng();

    let x = window_size.x / 2.;
    let y = window_size.y / 2.;

    Vec2::new(rng.gen_range(-x .. x), rng.gen_range(-y .. y))
}

#[derive(Component, Clone, Copy)]
struct Target(Vec2);

fn has_target(In(entity): In<Entity>, target_query: Query<(), With<Target>>) -> Status {
    match target_query.get(entity) {
        Ok(_) => Status::Success,
        Err(_) => Status::Failure,
    }
}

fn pick_target(In(entity): In<Entity>, mut commands: Commands, window: Query<&Window>) -> Status {
    let window = window.single();
    let x = window.width();
    let y = window.height();

    let window_size = Vec2::new(x, y);
    
    let target = random_point(window_size);

    commands.entity(entity).insert(Target(target));
    
    Status::Success
}

fn walk_to_target(In(entity): In<Entity>, mut transforms: Query<(&mut Transform, &Target), With<Agent>>, mut commands: Commands) -> Option<Status> {
    let (mut transform, target) = transforms.get_mut(entity).ok()?;

    let direction = target.0 - transform.translation.xy();
    let distance = direction.length();

    Some(
        if distance <= f32::EPSILON {
            commands.entity(entity).remove::<Target>();
            Status::Success
        } else {
            transform.translation += (direction.normalize() * 10.).clamp_length_max(distance).extend(0.);
            Status::Running
        }
    )
}

fn visualize(
    mut gizmos: Gizmos,
    query: Query<(&Transform, Option<&Target>), With<Agent>>,
) {
    for (transform, maybe_target) in query.iter() {
        let translation = transform.translation.xy();
        gizmos.circle_2d(translation, 5., Color::GREEN);

        if let Some(target) = maybe_target {
            gizmos.line_2d(translation, target.0, Color::RED);
            gizmos.circle_2d(target.0, 2., Color::RED);
        };
    }
}