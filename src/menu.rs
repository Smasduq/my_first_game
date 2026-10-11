use bevy::audio::{AudioSink, AudioSinkPlayback};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button, observe};

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
    Settings,
    Back,
    MusicToggle,
    SfxToggle,
}

const BTN_BG: Color = Color::srgb(0.16, 0.12, 0.17);
const BTN_HOVER: Color = Color::srgb(0.25, 0.16, 0.20);
const BTN_PRIMARY: Color = Color::srgb(0.91, 0.62, 0.26);
const BTN_PRIMARY_HOVER: Color = Color::srgb(0.98, 0.72, 0.35);
const CHARCOAL: Color = Color::srgb(0.09, 0.07, 0.11);
const INK: Color = Color::srgb(0.94, 0.91, 0.89);
const INK_DIM: Color = Color::srgb(0.78, 0.73, 0.77);
const BLOOD: Color = Color::srgb(0.75, 0.20, 0.22);

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
                    style_buttons.run_if(in_state(GameState::Menu)),
                    sync_settings_labels.run_if(in_state(GameState::Menu)),
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

fn button(label: &str, kind: MenuButton) -> impl Bundle {
    let text_color = if kind == MenuButton::Start {
        CHARCOAL
    } else {
        INK
    };
    (
        Button,
        Hovered::default(),
        kind,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(52.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(BTN_BG),
        observe(on_button_activate),
        children![(
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(text_color),
        )],
    )
}

fn build_menu(mut commands: Commands, screen: Res<MenuScreen>) {
    spawn_panel(&mut commands, *screen);
}

/// Rebuild the menu when switching panels.
fn rebuild(
    commands: &mut Commands,
    roots: &Query<Entity, With<MenuRoot>>,
    screen: &mut MenuScreen,
    next: MenuScreen,
) {
    for e in roots {
        commands.entity(e).despawn();
    }
    *screen = next;
    spawn_panel(commands, next);
}

fn spawn_panel(commands: &mut Commands, screen: MenuScreen) {
    let (node, bg) = overlay();
    match screen {
        MenuScreen::Main => {
            commands.spawn((
                MenuRoot,
                node,
                bg,
                children![(
                    Node {
                        width: Val::Percent(90.0),
                        max_width: Val::Px(360.0),
                        padding: UiRect::all(Val::Px(28.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(18.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.09, 0.07, 0.11, 0.96)),
                    children![
                        (
                            Text::new("DUSKFALL"),
                            TextFont {
                                font_size: FontSize::Px(56.0),
                                ..default()
                            },
                            TextColor(BLOOD),
                        ),
                        (
                            Text::new("OUTRUN THE EYES. REACH THE FLAG."),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(INK_DIM),
                        ),
                        button("PLAY", MenuButton::Start),
                        button("SETTINGS", MenuButton::Settings),
                        (
                            Text::new("A / D MOVE    SPACE JUMP    J FIRE"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(INK_DIM),
                        ),
                    ],
                )],
            ));
        }
        MenuScreen::Settings => {
            commands.spawn((
                MenuRoot,
                node,
                bg,
                children![(
                    Node {
                        width: Val::Percent(90.0),
                        max_width: Val::Px(360.0),
                        padding: UiRect::all(Val::Px(28.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(18.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.09, 0.07, 0.11, 0.96)),
                    children![
                        (
                            Text::new("SETTINGS"),
                            TextFont {
                                font_size: FontSize::Px(42.0),
                                ..default()
                            },
                            TextColor(BLOOD),
                        ),
                        button("MUSIC: ON", MenuButton::MusicToggle),
                        button("SOUND: ON", MenuButton::SfxToggle),
                        button("BACK", MenuButton::Back),
                    ],
                )],
            ));
        }
    }
}

fn tear_down_menu(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}

/// Restyle buttons from hover/press state every frame.
#[allow(clippy::type_complexity)]
fn style_buttons(
    mut btns: Query<(&MenuButton, &Hovered, Has<Pressed>, &mut BackgroundColor), With<Button>>,
) {
    for (kind, hovered, pressed, mut bg) in &mut btns {
        if *kind == MenuButton::Start {
            *bg = BackgroundColor(if pressed || hovered.get() {
                BTN_PRIMARY_HOVER
            } else {
                BTN_PRIMARY
            });
        } else if pressed {
            *bg = BackgroundColor(BLOOD);
        } else if hovered.get() {
            *bg = BackgroundColor(BTN_HOVER);
        } else {
            *bg = BackgroundColor(BTN_BG);
        }
    }
}

/// One observer shared by every button; the clicked kind is read
/// off the activated entity.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn on_button_activate(
    activate: On<Activate>,
    mut commands: Commands,
    roots: Query<Entity, With<MenuRoot>>,
    kinds: Query<&MenuButton>,
    mut screen: ResMut<MenuScreen>,
    mut state: ResMut<NextState<GameState>>,
    mut sfx: ResMut<SfxEnabled>,
    mut music: Query<&mut AudioSink, With<MusicTrack>>,
    mut level: ResMut<LevelState>,
    mut stats: ResMut<CombatStats>,
    spawn: Res<SpawnPoint>,
    mut player: Query<(&mut Transform, &mut Velocity, &mut Health), With<Player>>,
    strays: Query<Entity, Or<(With<Eye>, With<Bullet>, With<Life>)>>,
) {
    let Ok(kind) = kinds.get(activate.entity) else {
        return;
    };
    // Label text lives in the button's first child.
    match *kind {
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
        MenuButton::Settings => rebuild(&mut commands, &roots, &mut screen, MenuScreen::Settings),
        MenuButton::Back => rebuild(&mut commands, &roots, &mut screen, MenuScreen::Main),
        MenuButton::MusicToggle => {
            // Mute in place; unmute restores the volume.
            for mut sink in &mut music {
                sink.toggle_mute();
            }
        }
        MenuButton::SfxToggle => {
            sfx.0 = !sfx.0;
        }
    }
}

fn sync_settings_labels(
    music: Query<&AudioSink, With<MusicTrack>>,
    sfx: Res<SfxEnabled>,
    buttons: Query<(&MenuButton, &Children)>,
    mut texts: Query<&mut Text>,
) {
    let music_on = music.iter().all(|sink| !sink.is_muted());
    for (kind, children) in &buttons {
        let label = match kind {
            MenuButton::MusicToggle => Some(if music_on { "MUSIC: ON" } else { "MUSIC: OFF" }),
            MenuButton::SfxToggle => Some(if sfx.0 { "SOUND: ON" } else { "SOUND: OFF" }),
            _ => None,
        };
        let Some(label) = label else { continue };
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child)
                && **text != label
            {
                **text = label.into();
            }
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
