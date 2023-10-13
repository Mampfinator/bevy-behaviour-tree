//! I'd like for the API to be (somewhat) easily user-extendable. 
//! This test is to ensure that there's always a way to do that.
// TODO: this is currently less than ideal, since it requires a feature flag *and* type shenanigans with markers.
#![feature(return_position_impl_trait_in_trait)]
use bevy::prelude::{In, Entity};
use bevy_behaviour_tree::{prelude::*, TodoBehaviour, behaviour::{IntoBehaviour, SelfMarker}};

trait DecoratorExtensions<Marker> {
    fn extended(self) -> impl Behaviour + IntoBehaviour<SelfMarker>;
}

impl<Marker, T: Decorator<Marker>> DecoratorExtensions<Marker> for T {
    fn extended(self) -> impl Behaviour + IntoBehaviour<SelfMarker> {
        TodoBehaviour
    }
}

fn test_behaviour(_: In<Entity>) -> Status {
    Status::Failure
}

#[test]
fn test() {
    let mut trees = BehaviourTrees::default();
    let succeeding = test_behaviour.extended();
    trees.create(succeeding);
}