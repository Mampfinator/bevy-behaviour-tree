use bevy::{prelude::*, app::AppExit};
use bevy_behaviour_tree::prelude::*;

#[test]
fn main() {
    let mut ticks = 0;

    App::new().add_plugins((MinimalPlugins, BehaviourTreePlugin::default()))
        .add_systems(Startup, system)
        .add_systems(Update, move |mut quit: EventWriter<AppExit>| {
            ticks += 1;
            if ticks >= 500 {
                quit.send(AppExit);
            }
        })
        .run();
}


fn system(
    mut trees: ResMut<BehaviourTrees>,
    mut commands: Commands,
) {
    let id = trees.add((fail, succeed).chain());

    for _ in 0..100 {
        commands.spawn(id);
    }
}

fn fail(_: In<Entity>) -> Status {
    Status::Failure
}

fn succeed(_: In<Entity>) -> Status {
    Status::Success
}