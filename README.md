# bevy-behaviour-tree
A (prototype) crate that implements easy-to-use behaviour trees for [bevy](https://github.com/bevyengine/bevy), based on standard systems.


## Usage
As per usual, most functionality is exposed through a plugin:
```rust
// by default, runs in Update
app.insert_plugins(BehaviourTreePlugin::in_schedule(PostUpdate)); 
```

To use, just get the `BehaviourTrees` resource, and call `create`:
```rust
fn system(
    mut commands: Commands,
    mut trees: ResMut<BehaviourTrees>,
) {
    let behaviour = trees.create((
        (has_target, pick_target).select(), // read as "if we have a target, continue. If not, pick one, then continue."
        walk_to_target,
    ).sequence());

    commands.spawn((TransformBundle::default(), behaviour))
}
```
Any system that takes in an `Entity` and outputs a `Status` is automatically a valid node:

```rust
fn walk_to_target(In(entity): In<Entity>, mut transforms: Query<(&mut Transform, &Target)>, mut commands: Commands) -> Status {
    let Ok((mut transform, target)) = transforms.get_mut(entity) else { // curious? have a look at examples/moving_points.rs: there's a shorter syntax for this!
        return Status::Failure
    };

    let direction = target.0 - transform.translation.xy();
    let distance = direction.length();

    if distance <= f32::EPSILON {
        commands.entity(entity).remove::<Target>();
        Status::Success
    } else {
        transform.translation += (direction.normalize() * 10.).clamp_length_max(distance).extend(0.);
        Status::Running
    }
}
```