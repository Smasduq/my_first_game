use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaulPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(bundle: Camera2d);

}
