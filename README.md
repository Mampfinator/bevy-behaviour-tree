# bevy-behaviour-tree
A crate that implements easy-to-use behaviour trees for [Bevy](https://github.com/bevyengine/bevy), based on standard systems.

> :warning: this crate is currently more of a proof of concept.
> It "works" but performance and correctness is not guaranteed.

## Usage
As per usual, most functionality is exposed through a plugin:
```rust
// by default, runs in Update
app.insert_plugins(BehaviourTreePlugin::in_schedule(PostUpdate)); 
```

Call `create` on the `BehaviourTrees` resource to get a `BehaviourId`:
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
Everything with that `BehaviourId` will now act according to what you passed to `create`. Simple as that!

### Defining Behaviours
A "Behaviour" in this crate is anything that takes in an `Entity` and produces a `Status`.

Most importantly, any system that fits the bill is also a behaviour:
```rust
fn walk_to_target(In(entity): In<Entity>, mut transforms: Query<(&mut Transform, &Target)>, mut commands: Commands) -> Status {
    let Ok((mut transform, target)) = transforms.get_mut(entity) else {
        return Status::Failure
    };

    let direction = target.0 - transform.translation.xy();
    let distance = direction.length();

    if distance <= f32::EPSILON {
        commands.entity(entity).remove::<Target>();
        Status::Success
    } else {
        transform.translation += (direction.normalize() * 10.0.min(distance)).extend(0.);
        Status::Running
    }
}
```

### Control flow
You might've already spotted them up there - but `bevy-behaviour-tree` provides two built-in (and pretty standard) control flow concepts:

#### `sequence`
Does exactly what it says! It runs whatever it's called on in a sequence, from left to right. If anything in the sequence fails, the sequence itself fails (and resets). It succeeds when all of its inputs succeed.

For example:
```rust
(maybe_succeed, never_succeed, always_succeed).sequence()
```
If `maybe_succeed` fails, sequence fails. If it succeeds, sequence executes `never_succeed` next. `always_succeed` is never executed, since sequence always fails at `never_succeed`!

#### `select`
`select` is a bit more complicated. If you think of `sequence` as *AND*-ing its inputs together, `select` *OR*s them. It runs what it's called on from left to right and succeeds on the first successful input. It fails when none of its inputs succeed.
```
(maybe_succeed, never_succeed, always_succeed).select()
```
If `maybe_succeed` succeeds, great - `select` succeeds as well. If it fails, it tries `never_succeed`, gets a failure back and then moves on to `always_succeed`.


### Transforming output
`Decorators`, as they're called internally, transform output! They can be called on any single node in a tree.

Let's illustrate some examples.

Take the behaviour:
```rust
// you can return bools as status as well. true encodes success, false encodes failure.
fn has_target(In(entity): In<Entity>, query: Query<(), With<Target>>) -> bool {
    query.contains(entity)
}

// inverts the output; Success fails and failure succeeds!
let inverted = has_target.invert();

// this will retry for 10 ticks, and fail only if it doesn't succeed once
let retried = has_target.retry(10)

// this will retry forever, essentially blocking the tree until a target has been found.
// there's no shortcut for this yet.
let ad_infinitum = has_target.retry_while(|_: In<Entity>| true);
```