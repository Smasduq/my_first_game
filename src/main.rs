use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};
use bevy::prelude::*;

mod combat;
mod level;
mod menu;
mod player;

use combat::{CombatPlugin, CombatStats};
use level::{LevelState, TOTAL_CHECKPOINTS};
use menu::{GameState, Hud, MenuPlugin, MusicTrack};
use player::{Health, Player};

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct WinText;

/// Big rank letter (S / A / B / C).
#[derive(Component)]
struct WinRank;

/// Headline under the rank ("FLAWLESS ESCAPE!" / "YOU ESCAPED").
#[derive(Component)]
struct WinTitle;

/// Falling celebration squares on the win screen.
#[derive(Component)]
struct WinConfetti {
    fall: f32,
    sway: f32,
    phase: f32,
    spin: f32,
    x0: f32,
}

/// Tracks whether celebration FX has been spawned for the current win.
#[derive(Resource, Default)]
struct WonFx {
    done: bool,
}

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
        .init_resource::<WonFx>()
        .add_systems(Startup, (setup_camera, setup_ui, start_music))
        .add_systems(
            Update,
            (
                follow_camera,
                update_status,
                update_win,
                spawn_win_confetti,
                fall_win_confetti,
                pulse_win,
            ),
        )
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
            width: Val::Px(380.0),
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
                Text::new("CP 1 / 10     FALLS   0       EYES DOWN   0"),
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
        BackgroundColor(Color::srgba(0.01, 0.01, 0.03, 0.62)),
        Visibility::Hidden,
        children![(
            Node {
                width: Val::Percent(90.0),
                max_width: Val::Px(480.0),
                padding: UiRect::all(Val::Px(30.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(HUD_PANEL),
            children![
                (
                    Text::new("DUSKFALL  //  ESCAPE COMPLETE"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(HUD_GOLD),
                ),
                (
                    WinRank,
                    Text::new("S"),
                    TextFont {
                        font_size: FontSize::Px(72.0),
                        ..default()
                    },
                    TextColor(HUD_GOLD),
                ),
                (
                    WinTitle,
                    Text::new("YOU ESCAPED"),
                    TextFont {
                        font_size: FontSize::Px(38.0),
                        ..default()
                    },
                    TextColor(HUD_INK),
                ),
                (
                    Text::new("The last eye falls behind. Dawn is yours, runner."),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(HUD_MUTED),
                ),
                (
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Px(3.0),
                        margin: UiRect::axes(Val::Px(0.0), Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(HUD_GOLD),
                ),
                (
                    WinSummary,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(HUD_INK),
                ),
                (
                    Text::new("R  RUN AGAIN     ESC  MENU"),
                    TextFont {
                        font_size: FontSize::Px(14.0),
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
            "CP {} / {}     FALLS   {}       EYES DOWN   {}",
            state.current, TOTAL_CHECKPOINTS, state.deaths, combat.kills
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

/// Grade the run: no falls + high HP is S, more falls drop the rank.
fn rank_for(deaths: u32, hp: f32) -> (&'static str, Color, &'static str) {
    if deaths == 0 && hp >= 70.0 {
        (
            "S",
            Color::srgb(1.0, 0.80, 0.30),
            "FLAWLESS ESCAPE!",
        )
    } else if deaths <= 1 {
        ("A", Color::srgb(0.55, 0.95, 0.60), "YOU ESCAPED!")
    } else if deaths <= 3 {
        ("B", Color::srgb(0.55, 0.80, 1.0), "YOU MADE IT OUT!")
    } else {
        ("C", Color::srgb(0.85, 0.75, 0.85), "ESCAPED — BARELY!")
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn update_win(
    state: Res<LevelState>,
    game: Res<State<GameState>>,
    combat: Res<CombatStats>,
    health: Query<&Health, With<Player>>,
    mut vis: Query<&mut Visibility, With<WinText>>,
    mut summary: Query<&mut Text, With<WinSummary>>,
    mut rank: Query<&mut Text, (With<WinRank>, Without<WinSummary>, Without<WinTitle>)>,
    mut rank_color: Query<&mut TextColor, With<WinRank>>,
    mut title: Query<&mut Text, (With<WinTitle>, Without<WinSummary>, Without<WinRank>)>,
) {
    let Ok(mut v) = vis.single_mut() else { return };
    // Only celebrate mid-run; the pause menu has its own overlay.
    let show = state.won && **game == GameState::Playing;
    *v = if show {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if show
        && let (Ok(health), Ok(mut text)) = (health.single(), summary.single_mut())
    {
        let hp = health.hp.clamp(0.0, Health::MAX);
        let (letter, color, headline) = rank_for(state.deaths, hp);
        **text = format!(
            "ALL {} CHECKPOINTS REACHED     HEALTH  {:.0}%     FALLS  {}     EYES DOWN  {}",
            TOTAL_CHECKPOINTS, hp, state.deaths, combat.kills
        );
        if let Ok(mut r) = rank.single_mut()
            && **r != letter
        {
            **r = letter.into();
        }
        if let Ok(mut c) = rank_color.single_mut() {
            c.0 = color;
        }
        if let Ok(mut t) = title.single_mut()
            && **t != headline
        {
            **t = headline.into();
        }
    }
}

/// One-shot burst of falling confetti squares over the win panel.
fn spawn_win_confetti(
    mut commands: Commands,
    state: Res<LevelState>,
    game: Res<State<GameState>>,
    mut fx: ResMut<WonFx>,
) {
    let active = state.won && **game == GameState::Playing;
    if !active {
        return;
    }
    if fx.done {
        return;
    }
    fx.done = true;
    // Festive palette on the dark backdrop.
    let colors = [
        Color::srgb(1.0, 0.80, 0.30),
        Color::srgb(0.86, 0.25, 0.28),
        Color::srgb(0.55, 0.95, 0.60),
        Color::srgb(0.55, 0.80, 1.0),
        Color::srgb(0.94, 0.91, 0.89),
    ];
    for i in 0..110 {
        let f = i as f32;
        let color = colors[i % colors.len()];
        commands.spawn((
            WinConfetti {
                fall: 130.0 + (f * 37.0 % 160.0),
                sway: 30.0 + (f * 23.0 % 50.0),
                phase: f * 0.35,
                spin: 1.5 + (f * 13.0 % 3.0),
                x0: (f * 73.0 % 100.0),
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(f * 73.0 % 100.0),
                top: Val::Px(-40.0 - (f * 41.0 % 500.0)),
                width: Val::Px(6.0 + (f * 29.0 % 7.0)),
                height: Val::Px(8.0 + (f * 31.0 % 8.0)),
                ..default()
            },
            BackgroundColor(color),
        ));
    }
}

fn fall_win_confetti(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<LevelState>,
    game: Res<State<GameState>>,
    mut fx: ResMut<WonFx>,
    mut confetti: Query<(Entity, &mut Node, &mut Transform, &WinConfetti)>,
) {
    let active = state.won && **game == GameState::Playing;
    if !active {
        if fx.done {
            for (e, _, _, _) in &confetti {
                commands.entity(e).despawn();
            }
            fx.done = false;
        }
        return;
    }
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    for (_, mut node, mut transform, c) in &mut confetti {
        let Val::Px(top) = node.top else { continue };
        let mut next = top + c.fall * dt;
        // Recycle past the bottom of a typical window.
        if next > 1100.0 {
            next = -40.0;
        }
        node.top = Val::Px(next);
        node.left = Val::Percent(c.x0 + (t * 1.4 + c.phase).sin() * c.sway / 10.0);
        transform.rotation *= Quat::from_rotation_z(c.spin * dt);
    }
}

/// Gentle pulse on the rank + headline while celebrating.
fn pulse_win(
    time: Res<Time>,
    state: Res<LevelState>,
    mut ranks: Query<&mut Transform, With<WinRank>>,
    mut titles: Query<&mut TextColor, With<WinTitle>>,
) {
    if !state.won {
        return;
    }
    let t = time.elapsed_secs();
    let s = 1.0 + (t * 3.0).sin() * 0.06;
    for mut tr in &mut ranks {
        tr.scale = Vec3::new(s, s, 1.0);
    }
    // Shimmer the headline between ink and gold.
    let k = ((t * 2.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    let gold = Vec3::new(0.91, 0.62, 0.26);
    let ink = Vec3::new(0.94, 0.91, 0.89);
    let mix = ink.lerp(gold, k * 0.65);
    for mut c in &mut titles {
        c.0 = Color::srgb(mix.x, mix.y, mix.z);
    }
}
