use bevy::prelude::*;

use crate::level::{Goal, KILL_Y, LevelState, Platform, SpawnPoint};
use crate::menu::{GameState, SfxEnabled};

/// Marker for the player entity.
#[derive(Component)]
pub struct Player;

/// Vertical/horizontal speed.
#[derive(Component, Default)]
pub(crate) struct Velocity(pub(crate) Vec2);

/// Health pool. `invuln` is ticking i-frames after a hit.
#[derive(Component)]
pub(crate) struct Health {
    pub hp: f32,
    pub invuln: Timer,
}

impl Health {
    pub const MAX: f32 = 100.0;
    pub const INVULN_TIME: f32 = 1.0;

    pub(crate) fn full() -> Self {
        // Finished timer = vulnerable; `reset()` on a hit starts i-frames.
        let mut invuln = Timer::from_seconds(Self::INVULN_TIME, TimerMode::Once);
        invuln.finish();
        Self {
            hp: Self::MAX,
            invuln,
        }
    }
}

/// True when standing on a platform.
#[derive(Component, Default)]
struct Grounded(bool);

/// Side-view facing only.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FacingLR {
    Left,
    #[default]
    Right,
}

/// Space-marine frame set. Frames are individual images, swapped per tick.
#[derive(Component)]
struct MarineAnim {
    run: Vec<Handle<Image>>,
    idle: Handle<Image>,
    jump: Handle<Image>,
    frame: usize,
    timer: Timer,
}

/// Muzzle position relative to player center, by facing.
pub(crate) fn muzzle_offset(f: FacingLR) -> Vec2 {
    let s = match f {
        FacingLR::Right => 1.0,
        FacingLR::Left => -1.0,
    };
    Vec2::new(62.0 * s, 6.0)
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player).add_systems(
            Update,
            (
                run_jump_physics,
                animate_marine,
                check_fall,
                check_goal,
                restart_on_key,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}

const MARINE_BASE: &str = "Warped/Characters/space-marine-lite/Sprites";

const GRAVITY: f32 = 2600.0;
const JUMP_VEL: f32 = 950.0;
const RUN_SPEED: f32 = 330.0;
const MAX_FALL: f32 = 1400.0;

/// Marine is 75x48 at 2x. Tight box so bullets feel fair.
const PLAYER_HALF: Vec2 = Vec2::new(20.0, 42.0);

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>, spawn: Res<SpawnPoint>) {
    let run = (1..=10)
        .map(|i| {
            asset_server.load(format!(
                "{MARINE_BASE}/Run with Gun/sprites/run-with-gun{i}.png"
            ))
        })
        .collect();
    let idle: Handle<Image> =
        asset_server.load(format!("{MARINE_BASE}/Idle Gun/sprites/idle-gun1.png"));
    let jump: Handle<Image> = asset_server.load(format!(
        "{MARINE_BASE}/Jump with Gun/sprites/jump-with-gun3.png"
    ));

    commands.spawn((
        Player,
        Velocity::default(),
        Health::full(),
        Grounded(false),
        FacingLR::Right,
        MarineAnim {
            run,
            idle: idle.clone(),
            jump,
            frame: 0,
            timer: Timer::from_seconds(0.08, TimerMode::Repeating),
        },
        Sprite {
            image: idle,
            ..default()
        },
        Transform {
            translation: spawn.0,
            scale: Vec3::new(2.0, 2.0, 1.0),
            ..default()
        },
    ));
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn run_jump_physics(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    state: Res<LevelState>,
    audio: Res<AssetServer>,
    sfx: Res<SfxEnabled>,
    mut commands: Commands,
    mut player: Query<
        (&mut Transform, &mut Velocity, &mut Grounded, &mut FacingLR),
        (With<Player>, Without<Platform>),
    >,
    platforms: Query<(&Transform, &Platform), Without<Player>>,
) {
    let Ok((mut transform, mut vel, mut grounded, mut facing)) = player.single_mut() else {
        return;
    };
    if state.won {
        vel.0 = Vec2::ZERO;
        return;
    }

    let dt = time.delta_secs();
    let mut dir = 0.0;
    if input.pressed(KeyCode::ArrowLeft) || input.pressed(KeyCode::KeyA) {
        dir -= 1.0;
    }
    if input.pressed(KeyCode::ArrowRight) || input.pressed(KeyCode::KeyD) {
        dir += 1.0;
    }
    if dir > 0.0 {
        *facing = FacingLR::Right;
    } else if dir < 0.0 {
        *facing = FacingLR::Left;
    }
    vel.0.x = dir * RUN_SPEED;

    let jump_pressed = input.just_pressed(KeyCode::Space)
        || input.just_pressed(KeyCode::KeyW)
        || input.just_pressed(KeyCode::ArrowUp);
    if jump_pressed && grounded.0 {
        vel.0.y = JUMP_VEL;
        grounded.0 = false;
        if sfx.0 {
            commands.spawn(AudioPlayer::new(
                audio.load("12_Player_Movement_SFX/30_Jump_03.ogg"),
            ));
        }
    }

    vel.0.y -= GRAVITY * dt;
    vel.0.y = vel.0.y.max(-MAX_FALL);

    // Move X, then resolve.
    transform.translation.x += vel.0.x * dt;
    for (pt, p) in &platforms {
        if overlap(
            transform.translation.truncate(),
            PLAYER_HALF,
            pt.translation.truncate(),
            p.size * 0.5,
        ) {
            if vel.0.x > 0.0 {
                transform.translation.x = pt.translation.x - p.size.x * 0.5 - PLAYER_HALF.x;
            } else if vel.0.x < 0.0 {
                transform.translation.x = pt.translation.x + p.size.x * 0.5 + PLAYER_HALF.x;
            }
            vel.0.x = 0.0;
        }
    }

    // Move Y, then resolve.
    let prev_bottom = transform.translation.y - PLAYER_HALF.y;
    transform.translation.y += vel.0.y * dt;
    grounded.0 = false;
    for (pt, p) in &platforms {
        if overlap(
            transform.translation.truncate(),
            PLAYER_HALF,
            pt.translation.truncate(),
            p.size * 0.5,
        ) {
            let plat_top = pt.translation.y + p.size.y * 0.5;
            if vel.0.y <= 0.0 && prev_bottom >= plat_top - 12.0 {
                transform.translation.y = plat_top + PLAYER_HALF.y;
                vel.0.y = 0.0;
                grounded.0 = true;
            } else if vel.0.y > 0.0 {
                let plat_bottom = pt.translation.y - p.size.y * 0.5;
                transform.translation.y = plat_bottom - PLAYER_HALF.y;
                vel.0.y = 0.0;
            }
        }
    }
}

fn overlap(c1: Vec2, h1: Vec2, c2: Vec2, h2: Vec2) -> bool {
    (c1.x - c2.x).abs() < h1.x + h2.x && (c1.y - c2.y).abs() < h1.y + h2.y
}

#[allow(clippy::type_complexity)]
fn animate_marine(
    time: Res<Time>,
    mut query: Query<
        (
            &FacingLR,
            &Velocity,
            &Grounded,
            &Health,
            &mut MarineAnim,
            &mut Sprite,
            &mut Transform,
        ),
        With<Player>,
    >,
) {
    for (facing, vel, grounded, health, mut anim, mut sprite, mut transform) in &mut query {
        // Marine art faces right; mirror for left.
        transform.scale.x = match facing {
            FacingLR::Right => 2.0,
            FacingLR::Left => -2.0,
        };
        transform.scale.y = 2.0;

        let running = grounded.0 && vel.0.x.abs() > 20.0;
        if !grounded.0 {
            sprite.image = anim.jump.clone();
            anim.timer.reset();
        } else if running {
            anim.timer.tick(time.delta());
            if anim.timer.just_finished() {
                anim.frame = (anim.frame + 1) % anim.run.len();
            }
            let img = anim.run[anim.frame].clone();
            sprite.image = img;
        } else {
            sprite.image = anim.idle.clone();
            anim.frame = 0;
            anim.timer.reset();
        }
        let blinking = !health.invuln.is_finished()
            && ((health.invuln.elapsed_secs() * 10.0).floor() as u32).is_multiple_of(2);
        sprite.color = if blinking {
            Color::srgb(1.0, 0.55, 0.55)
        } else {
            Color::WHITE
        };
    }
}

fn check_fall(
    mut player: Query<&mut Transform, With<Player>>,
    spawn: Res<SpawnPoint>,
    mut state: ResMut<LevelState>,
    mut stuff: Query<(&mut Velocity, &mut Health), With<Player>>,
) {
    let Ok(mut transform) = player.single_mut() else {
        return;
    };
    if transform.translation.y < KILL_Y {
        transform.translation = spawn.0;
        if let Ok((mut v, mut h)) = stuff.single_mut() {
            v.0 = Vec2::ZERO;
            *h = Health::full();
        }
        state.deaths += 1;
    }
}

fn check_goal(
    mut commands: Commands,
    audio: Res<AssetServer>,
    sfx: Res<SfxEnabled>,
    player: Query<&Transform, With<Player>>,
    goals: Query<&Transform, (With<Goal>, Without<Player>)>,
    mut state: ResMut<LevelState>,
) {
    if state.won {
        return;
    }
    let Ok(p) = player.single() else { return };
    for g in &goals {
        // Generous banner hitbox.
        if (p.translation.x - g.translation.x).abs() < 60.0
            && (p.translation.y - g.translation.y).abs() < 110.0
        {
            state.won = true;
            if sfx.0 {
                commands.spawn(AudioPlayer::new(
                    audio.load("10_Battle_SFX/55_Encounter_02.ogg"),
                ));
            }
            break;
        }
    }
}

fn restart_on_key(
    input: Res<ButtonInput<KeyCode>>,
    mut player: Query<&mut Transform, With<Player>>,
    mut stuff: Query<(&mut Velocity, &mut Health), With<Player>>,
    spawn: Res<SpawnPoint>,
    mut state: ResMut<LevelState>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        if let Ok(mut t) = player.single_mut() {
            t.translation = spawn.0;
        }
        if let Ok((mut v, mut h)) = stuff.single_mut() {
            v.0 = Vec2::ZERO;
            *h = Health::full();
        }
        state.won = false;
    }
}
