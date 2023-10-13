//! Showcases how to construct and use basic behaviour trees by spawning 100 agents that randomly walk across the screen.
#![allow(clippy::type_complexity)]
use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_behaviour_tree::prelude::*;
use rand::{thread_rng, Rng};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BehaviourTreePlugin::default()))
        .add_systems(Startup, (spawn_camera, spawn_agents))
        .add_systems(PreUpdate, visualize)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

/// A marker component so we can find our agents again.
#[derive(Component)]
struct Agent;

#[derive(Component)]
struct Speed(f32);

fn spawn_agents(mut commands: Commands, mut trees: ResMut<BehaviourTrees>, window: Query<&Window>) {
    let behaviour = trees.create(
        (
            (has_target, (wait(|| 1.), pick_target).sequence()).select(), // "select" short circuits on the first successful child; so if we have a target, we're done already,
            walk_to_target,
            wait(|| thread_rng().gen_range(0.5..3.0)),
        )
            .sequence(), // sequence does what you'd expect - it runs whatever it's called on in sequence.
    );

    let window = window.single();
    let x = window.width();
    let y = window.height();

    let window_size = Vec2::new(x, y);

    for _ in 0..500 {
        // the "BehaviourId" is just a cheap reference to the actual behaviour tree. We can copy it and stick it on as many agents as we want after creation.
        commands.spawn((
            Agent,
            random_transform(window_size),
            Speed(thread_rng().gen_range(3.0..7.0)),
            behaviour,
        ));
    }
}

#[derive(Component, Clone, Copy)]
struct Target(Vec2);

fn has_target(In(entity): In<Entity>, target_query: Query<(), With<Target>>) -> bool {
    target_query.contains(entity)
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

fn walk_to_target(
    In(entity): In<Entity>,
    mut query: Query<(&mut Transform, &Target, &Speed), With<Agent>>,
    mut commands: Commands,
) -> Option<Status> {
    let (mut transform, target, speed) = query.get_mut(entity).ok()?;

    let direction = target.0 - transform.translation.xy();
    let distance = direction.length();

    Some(if distance <= f32::EPSILON {
        commands.entity(entity).remove::<Target>();
        Status::Success
    } else {
        transform.translation += (direction.normalize() * speed.0.min(distance)).extend(0.);
        Status::Running
    })
}

#[derive(Component)]
struct Waiting(Timer);

fn wait(
    mut get_wait_time: impl FnMut() -> f32,
) -> impl FnMut(In<Entity>, Commands<'_, '_>, Query<'_, '_, &mut Waiting>, Res<'_, Time>) -> Status
{
    move |In(entity): In<Entity>,
          mut commands: Commands,
          mut wait: Query<&mut Waiting>,
          time: Res<Time>| {
        let Ok(mut wait) = wait.get_mut(entity) else {
            commands.entity(entity).insert(Waiting(Timer::from_seconds(
                get_wait_time(),
                TimerMode::Once,
            )));

            return Status::Running;
        };

        wait.0.tick(time.delta());

        if wait.0.finished() {
            commands.entity(entity).remove::<Waiting>();
            Status::Success
        } else {
            Status::Running
        }
    }
}

fn visualize(
    mut gizmos: Gizmos,
    query: Query<(&Transform, Option<&Target>, Option<&Waiting>), With<Agent>>,
) {
    for (transform, maybe_target, maybe_waiting) in query.iter() {
        let translation = transform.translation.xy();
        gizmos.circle_2d(
            translation,
            2.,
            if maybe_waiting.is_some() {
                Color::YELLOW
            } else {
                Color::GREEN
            },
        );

        if let Some(target) = maybe_target {
            gizmos.line_2d(translation, target.0, Color::RED);
            gizmos.circle_2d(target.0, 1., Color::RED);
        };
    }
}

fn random_transform(window_size: Vec2) -> TransformBundle {
    TransformBundle::from_transform(Transform::from_translation(
        random_point(window_size).extend(0.),
    ))
}

fn random_point(window_size: Vec2) -> Vec2 {
    let mut rng = thread_rng();

    let x = window_size.x / 2.;
    let y = window_size.y / 2.;

    Vec2::new(rng.gen_range(-x..x), rng.gen_range(-y..y))
}
