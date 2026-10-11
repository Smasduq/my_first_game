use bevy::audio::AudioSink;
use bevy::prelude::*;

use crate::combat::{Bullet, CombatStats, Eye, Life};
use crate::level::{LevelState, SpawnPoint};
use crate::player::{Health, Player, Velocity};

/// Top-level flow: menu overlay vs live gameplay.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum GameState {
    #[default]
    Menu,
    Playing,
}

/// Which menu panel is showing.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
enum MenuScreen {
    #[default]
    Main,
    Settings,
}

/// Master switch for one-shot SFX (music has its own toggle).
#[derive(Resource)]
pub(crate) struct SfxEnabled(pub bool);

/// Marker on the looping music entity.
#[derive(Component)]
pub(crate) struct MusicTrack;

/// Marker on all menu UI, despawned when leaving the menu.
#[derive(Component)]
struct MenuRoot;

/// Marker on HUD nodes, hidden while in the menu.
#[derive(Component)]
pub(crate) struct Hud;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum MenuButton {
    Start,
    LevelLocked,
    Settings,
    Back,
    MusicToggle,
    SfxToggle,
}

const BTN_BG: Color = Color::srgb(0.16, 0.08, 0.10);
const BTN_HOVER: Color = Color::srgb(0.35, 0.14, 0.16);
const BTN_LOCKED: Color = Color::srgb(0.08, 0.08, 0.10);
const INK: Color = Color::srgb(0.92, 0.82, 0.82);
const INK_DIM: Color = Color::srgb(0.45, 0.42, 0.45);
const BLOOD: Color = Color::srgb(0.75, 0.16, 0.16);

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .insert_resource(MenuScreen::Main)
            .insert_resource(SfxEnabled(true))
            .add_systems(OnEnter(GameState::Menu), build_menu)
            .add_systems(OnExit(GameState::Menu), tear_down_menu)
            .add_systems(
                Update,
                (
                    menu_buttons.run_if(in_state(GameState::Menu)),
                    esc_toggle,
                    sync_hud,
                ),
            );
    }
}

fn overlay() -> (Node, BackgroundColor) {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(14.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.01, 0.01, 0.03, 0.72)),
    )
}

fn button(label: &str, kind: MenuButton, w: f32) -> impl Bundle {
    (
        Button,
        kind,
        Node {
            width: Val::Px(w),
            height: Val::Px(52.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(BTN_BG),
        children![(
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(INK),
        )],
    )
}

fn build_menu(mut commands: Commands, screen: Res<MenuScreen>) {
    spawn_panel(&mut commands, *screen);
}

/// Rebuild the menu when switching panels.
fn rebuild(commands: &mut Commands, roots: &Query<Entity, With<MenuRoot>>, screen: &mut MenuScreen, next: MenuScreen) {
    for e in roots {
        commands.entity(e).despawn();
    }
    **screen = next;
    spawn_panel(commands, next);
}

fn spawn_panel(commands: &mut Commands, screen: MenuScreen) {
    let (node, bg) = overlay();
    match screen {
        MenuScreen::Main => {
            commands
                .spawn((MenuRoot, node, bg, children![
                    (
                        Text::new("DUSKFALL"),
                        TextFont {
                            font_size: FontSize::Px(84.0),
                            ..default()
                        },
                        TextColor(BLOOD),
                    ),
                    (
                        Text::new("outrun the eyes. reach the flag."),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(INK_DIM),
                    ),
                    // Level row.
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        children![
                            button("1", MenuButton::Start, 90.0),
                            button("2", MenuButton::LevelLocked, 90.0),
                            button("3", MenuButton::LevelLocked, 90.0),
                        ],
                    ),
                    button("SETTINGS", MenuButton::Settings, 260.0),
                    (
                        Text::new("A/D run - SPACE jump - J shoot - ESC pause"),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(INK_DIM),
                    ),
                ]));
        }
        MenuScreen::Settings => {
            commands
                .spawn((MenuRoot, node, bg, children![
                    (
                        Text::new("SETTINGS"),
                        TextFont {
                            font_size: FontSize::Px(56.0),
                            ..default()
                        },
                        TextColor(BLOOD),
                    ),
                    button("MUSIC: ON", MenuButton::MusicToggle, 300.0),
                    button("SOUND: ON", MenuButton::SfxToggle, 300.0),
                    button("BACK", MenuButton::Back, 300.0),
                ]));
        }
    }
}

fn tear_down_menu(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}

/// Button hover + click handling. Menu entities are despawned on
/// state exit, so labels are rebuilt fresh on each panel switch.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn menu_buttons(
    mut commands: Commands,
    roots: Query<Entity, With<MenuRoot>>,
    mut screen: ResMut<MenuScreen>,
    mut state: ResMut<NextState<GameState>>,
    mut sfx: ResMut<SfxEnabled>,
    music: Query<&AudioSink, With<MusicTrack>>,
    mut btns: Query<(&Interaction, &MenuButton, &mut BackgroundColor, &Children), Changed<Interaction>>,
    mut texts: Query<&mut Text>,
    mut level: ResMut<LevelState>,
    mut stats: ResMut<CombatStats>,
    spawn: Res<SpawnPoint>,
    mut player: Query<(&mut Transform, &mut Velocity, &mut Health), With<Player>>,
    strays: Query<Entity, Or<(With<Eye>, With<Bullet>, With<Life>)>>,
) {
    for (hit, kind, mut bg, children) in &mut btns {
        match *hit {
            Interaction::Hovered => {
                if !matches!(kind, MenuButton::LevelLocked) {
                    *bg = BackgroundColor(BTN_HOVER);
                }
            }
            Interaction::None => {
                *bg = BackgroundColor(match kind {
                    MenuButton::LevelLocked => BTN_LOCKED,
                    _ => BTN_BG,
                });
            }
            Interaction::Pressed => match *kind {
                MenuButton::Start => {
                    // Fresh run: reset world, clear leftovers.
                    level.won = false;
                    level.deaths = 0;
                    stats.kills = 0;
                    stats.hits = 0;
                    if let Ok((mut t, mut v, mut h)) = player.single_mut() {
                        t.translation = spawn.0;
                        v.0 = Vec2::ZERO;
                        *h = Health::full();
                    }
                    for e in &strays {
                        commands.entity(e).despawn();
                    }
                    state.set(GameState::Playing);
                }
                MenuButton::LevelLocked => {}
                MenuButton::Settings => rebuild(&mut commands, &roots, &mut screen, MenuScreen::Settings),
                MenuButton::Back => rebuild(&mut commands, &roots, &mut screen, MenuScreen::Main),
                MenuButton::MusicToggle => {
                    // Pause the loop in place so it resumes where it left off.
                    let mut on = false;
                    for sink in &music {
                        if sink.is_paused() {
                            sink.play();
                            on = true;
                        } else {
                            sink.pause();
                            on = false;
                        }
                    }
                    for child in children {
                        if let Ok(mut t) = texts.get_mut(*child) {
                            **t = if on { "MUSIC: ON".into() } else { "MUSIC: OFF".into() };
                        }
                    }
                }
                MenuButton::SfxToggle => {
                    sfx.0 = !sfx.0;
                    for child in children {
                        if let Ok(mut t) = texts.get_mut(*child) {
                            **t = if sfx.0 { "SOUND: ON".into() } else { "SOUND: OFF".into() };
                        }
                    }
                }
            },
        }
    }
}

/// ESC pauses to the menu, resumes, or steps out of settings.
fn esc_toggle(
    input: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut screen: ResMut<MenuScreen>,
    roots: Query<Entity, With<MenuRoot>>,
    mut commands: Commands,
) {
    if !input.just_pressed(KeyCode::Escape) {
        return;
    }
    match (**state, *screen) {
        (GameState::Playing, _) => next.set(GameState::Menu),
        (GameState::Menu, MenuScreen::Settings) => {
            rebuild(&mut commands, &roots, &mut screen, MenuScreen::Main)
        }
        (GameState::Menu, MenuScreen::Main) => next.set(GameState::Playing),
    }
}

/// HUD only makes sense mid-run.
fn sync_hud(state: Res<State<GameState>>, mut vis: Query<&mut Visibility, With<Hud>>) {
    let show = matches!(**state, GameState::Playing);
    for mut v in &mut vis {
        *v = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
