use bevy::prelude::*;
use bevy_behaviour_tree::prelude::*;

fn main() {
    App::new().add_plugins((MinimalPlugins, BehaviourTreePlugin::default()));
}
