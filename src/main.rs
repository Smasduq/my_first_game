use bevy::prelude::*;

mod combat;
mod level;
mod player;

use combat::{CombatPlugin, CombatStats};
use level::LevelState;
use player::Player;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct WinText;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.015, 0.015, 0.03)))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "src/assets".into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((level::LevelPlugin, player::PlayerPlugin, CombatPlugin))
        .add_systems(Startup, (setup_camera, setup_ui))
        .add_systems(Update, (follow_camera, update_status, update_win))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_ui(mut commands: Commands) {
    // Controls hint.
    commands.spawn((
        Text::new("A/D or Arrows: run   Space/W/Up: jump   J: shoot   R: restart"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgba(0.75, 0.72, 0.78, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
    // Deaths / status.
    commands.spawn((
        StatusText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgba(0.85, 0.6, 0.6, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(32.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
    // Center win message, hidden until won.
    commands.spawn((
        WinText,
        Text::new("YOU ESCAPED  (R to run again)"),
        TextFont {
            font_size: FontSize::Px(42.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.3, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(38.0),
            margin: UiRect {
                left: Val::Px(-260.0),
                ..default()
            },
            ..default()
        },
        Visibility::Hidden,
    ));
}

fn follow_camera(
    time: Res<Time>,
    player: Query<&Transform, With<Player>>,
    mut camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let Ok(p) = player.single() else { return };
    let Ok(mut cam) = camera.single_mut() else {
        return;
    };
    // Look slightly ahead and above the runner; smooth it.
    let target = Vec3::new(p.translation.x + 90.0, (p.translation.y + 90.0).max(-40.0), cam.translation.z);
    let k = 1.0 - (-5.0 * time.delta_secs()).exp();
    cam.translation = cam.translation.lerp(target, k);
}

fn update_status(
    state: Res<LevelState>,
    combat: Res<CombatStats>,
    mut text: Query<&mut Text, With<StatusText>>,
) {
    let Ok(mut t) = text.single_mut() else { return };
    if state.won {
        **t = format!(
            "Escaped! Falls: {}  Crows shot: {}",
            state.deaths, combat.kills
        );
    } else {
        **t = format!(
            "Reach the red flag. Falls: {}  Crows shot: {}  Hits taken: {}",
            state.deaths, combat.kills, combat.hits
        );
    }
}

fn update_win(state: Res<LevelState>, mut vis: Query<&mut Visibility, With<WinText>>) {
    let Ok(mut v) = vis.single_mut() else { return };
    *v = if state.won {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}
