use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};
use bevy::prelude::*;

mod combat;
mod level;
mod menu;
mod player;

use combat::{CombatPlugin, CombatStats};
use level::LevelState;
use menu::{Hud, MenuPlugin, MusicTrack};
use player::{Health, Player};

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct WinText;

#[derive(Component)]
struct HealthLabel;

#[derive(Component)]
struct HealthBarFill;

#[derive(Component)]
struct WinSummary;

const HUD_PANEL: Color = Color::srgba(0.09, 0.07, 0.11, 0.94);
const HUD_INK: Color = Color::srgb(0.94, 0.91, 0.89);
const HUD_MUTED: Color = Color::srgb(0.78, 0.73, 0.77);
const HUD_GOLD: Color = Color::srgb(0.91, 0.62, 0.26);
const HUD_DANGER: Color = Color::srgb(0.86, 0.22, 0.24);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.015, 0.015, 0.03)))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "src/assets".into(),
                    ..default()
                })
                // rodio logs backend buffer noise as ERROR every hitch;
                // silence the crate, game audio is unaffected.
                .set(bevy::log::LogPlugin {
                    filter: "info,rodio=off".into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((
            level::LevelPlugin,
            player::PlayerPlugin,
            CombatPlugin,
            MenuPlugin,
        ))
        .add_systems(Startup, (setup_camera, setup_ui, start_music))
        .add_systems(Update, (follow_camera, update_status, update_win))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Looping background track, kept under the SFX.
fn start_music(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        MusicTrack,
        AudioPlayer::new(assets.load("2023 _ Clement Panchout - Unrecord.ogg")),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.35),
            ..default()
        },
    ));
}

fn setup_ui(mut commands: Commands) {
    // Compact status panel.
    commands.spawn((
        Hud,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            left: Val::Px(14.0),
            width: Val::Px(310.0),
            padding: UiRect::all(Val::Px(14.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(9.0),
            ..default()
        },
        BackgroundColor(HUD_PANEL),
        children![
            (
                Text::new("DUSKFALL  /  VITALS"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(HUD_MUTED),
            ),
            (
                HealthLabel,
                Text::new("100 / 100"),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(HUD_INK),
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.20, 0.16, 0.21)),
                children![(
                    HealthBarFill,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(HUD_GOLD),
                )],
            ),
            (
                StatusText,
                Text::new("FALLS   0       EYES DOWN   0"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(HUD_INK),
            ),
        ],
    ));

    // Controls stay separate from run status.
    commands.spawn((
        Hud,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            left: Val::Px(14.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(HUD_PANEL),
        children![(
            Text::new("A / D  MOVE     SPACE  JUMP     J  FIRE     ESC  PAUSE"),
            TextFont {
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(HUD_MUTED),
        )],
    ));

    // Centered results panel, hidden until the finish.
    commands.spawn((
        WinText,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.01, 0.01, 0.03, 0.78)),
        Visibility::Hidden,
        children![(
            Node {
                width: Val::Percent(90.0),
                max_width: Val::Px(440.0),
                padding: UiRect::all(Val::Px(26.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(HUD_PANEL),
            children![
                (
                    Text::new("DUSKFALL"),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(HUD_GOLD),
                ),
                (
                    Text::new("YOU ESCAPED"),
                    TextFont {
                        font_size: FontSize::Px(38.0),
                        ..default()
                    },
                    TextColor(HUD_INK),
                ),
                (
                    WinSummary,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                    TextColor(HUD_MUTED),
                ),
                (
                    Text::new("R  RUN AGAIN     ESC  RETURN TO MENU"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(HUD_GOLD),
                ),
            ],
        )],
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
    let target = Vec3::new(
        p.translation.x + 90.0,
        (p.translation.y + 90.0).max(-40.0),
        cam.translation.z,
    );
    let k = 1.0 - (-5.0 * time.delta_secs()).exp();
    cam.translation = cam.translation.lerp(target, k);
}

fn update_status(
    state: Res<LevelState>,
    combat: Res<CombatStats>,
    health: Query<&Health, With<Player>>,
    mut text: Query<&mut Text, With<StatusText>>,
    mut label: Query<&mut Text, (With<HealthLabel>, Without<StatusText>)>,
    mut bar: Query<&mut Node, With<HealthBarFill>>,
    mut bar_color: Query<&mut BackgroundColor, With<HealthBarFill>>,
) {
    let Ok(health) = health.single() else { return };
    let hp = health.hp.clamp(0.0, Health::MAX);
    if let Ok(mut t) = text.single_mut() {
        **t = format!(
            "FALLS   {}       EYES DOWN   {}",
            state.deaths, combat.kills
        );
    }
    if let Ok(mut t) = label.single_mut() {
        **t = format!("{hp:.0} / {:.0}", Health::MAX);
    }
    if let Ok(mut node) = bar.single_mut() {
        node.width = Val::Percent(hp / Health::MAX * 100.0);
    }
    if let Ok(mut color) = bar_color.single_mut() {
        color.0 = if hp <= 25.0 { HUD_DANGER } else { HUD_GOLD };
    }
}

fn update_win(
    state: Res<LevelState>,
    combat: Res<CombatStats>,
    health: Query<&Health, With<Player>>,
    mut vis: Query<&mut Visibility, With<WinText>>,
    mut summary: Query<&mut Text, With<WinSummary>>,
) {
    let Ok(mut v) = vis.single_mut() else { return };
    *v = if state.won {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if state.won
        && let (Ok(health), Ok(mut text)) = (health.single(), summary.single_mut())
    {
        **text = format!(
            "HEALTH  {:.0}%     FALLS  {}     EYES DOWN  {}",
            health.hp, state.deaths, combat.kills
        );
    }
}
