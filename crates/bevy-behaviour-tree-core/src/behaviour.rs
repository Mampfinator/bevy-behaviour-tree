use bevy::prelude::{Entity, System, World, IntoSystem};

/// The trait at the core of this crate.
///
/// The idea is simple: a `Behaviour` takes in an [`Entity`] and the [`World`] it belongs to, along with its own arbitrary state, and returns a [`Status`], indicating whether it's running, has failed or succeeded.
///
/// The most important implementation for `Behaviour` is a blanket implementation for any [`System<In = Entity, Out = Status>`][bevy::ecs::system::System],
/// meaning that any user-defined system that takes in an `Entity` and returns a `Status` is automatically a `Behaviour`.
/// If you've never seen or used system inputs before, have a look at [`In`] and the [piping example](https://github.com/bevyengine/bevy/blob/main/examples/ecs/system_piping.rs).
///
/// There are three basic types of behaviours:
///  - *Leafs*: they access and/or modify world state directly. These are usually user defined, like a system to make an entity walk from A to B, or to check if there are enemies nearby.
///  - *Decorators*: they modify the output of another behaviour, like [`invert`][DecoratorInput::invert] and [`retry_while`][DecoratorInput::retry_while].
///  - *Compositors*: they modify the output of a group of other behaviours, like [`select`] and [`chain`]
///
/// These types aren't strictly enforced, but are the defacto standard implementation for behaviour tree nodes. You can freely extend and mix them as you see fit, by using the aforementioned system piping for example.
/// As long as the resulting system takes in an `Entity` and outputs a `Status`, it's a valid `Behaviour` usable with this crate.
///
/// For more complex custom implementations, you need to make sure that all underlying systems [initialize][bevy::ecs::system::System::initialize] correcly.
/// If you see a `Encountered a mismathed World.` panic, it's likely one of the systems you rely on wasn't initialized properly.
pub trait Behaviour: Send + Sync + 'static {
    /// Runs the behaviour on the given entity, in the given world. Called once a world tick at most.
    ///
    /// # Panics
    /// If the world passed is not the same one that was passed in [`initialize`][Behaviour::initialize].
    fn run(&mut self, entity: Entity, world: &mut World) -> Status;

    /// Initializes the behaviour. This registers component access for underlying systems, and does general setup work.
    /// Required to be called before [`run`][Behaviour::run].
    fn initialize(&mut self, world: &mut World);
}

/// The status of a [`Behaviour`], returned when it's [`run`][Behaviour::run].
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Status {
    /// Indicates a successful action.
    Success,
    /// Indicates a failed action.
    Failure,
    /// Indicates that an action requires more time to complete.
    Running,
}

struct SystemBehaviour<F>
where 
    F: System<In = Entity, Out = Status> {
        func: F,
}

impl<F> Behaviour for SystemBehaviour<F>
where 
    F: System<In = Entity, Out = Status> {
    fn initialize(&mut self, world: &mut World) {
        self.func.initialize(world)
    }
    
    fn run(&mut self, entity: Entity, world: &mut World) -> Status {
        self.func.run(entity, world)
    }
}

/// Conversion trait for behaviours.
pub trait IntoBehaviour<Marker> {
    /// Conversion function.
    fn into_behaviour(self) -> impl Behaviour;
}

impl<Marker: 'static, T> IntoBehaviour<Marker> for T
where
    T: IntoSystem<Entity, Status, Marker>,
{
    fn into_behaviour(self) -> impl Behaviour {
        SystemBehaviour {
            func: IntoSystem::into_system(self)
        }
    }
}