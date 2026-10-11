use bevy::prelude::*;

#[derive(Component)]
struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
            Text2d::new("@"),
            TextFont {
                font_size: FontSize::Px(12.0),
                font: default(),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_translation(Vec3::ZERO),
            Player
            ));

}
