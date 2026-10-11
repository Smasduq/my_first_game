use bevy::prelude::*;

use crate::level::LevelState;
use crate::player::{muzzle_offset, FacingLR, Player};

/// Kills by gunfire + times a crow reached the player.
#[derive(Resource, Default)]
pub struct CombatStats {
    pub kills: u32,
    pub hits: u32,
}

/// A crow flying at the player. `vel` is fixed at spawn plus a wobble.
#[derive(Component)]
struct Crow {
    vel: Vec2,
    phase: f32,
}

/// Crow wing-flap state (2-frame atlas).
#[derive(Component)]
struct CrowFlap {
    timer: Timer,
}

/// Bullet in flight.
#[derive(Component)]
struct Bullet {
    vel: Vec2,
}

/// Fired every few seconds while the level runs.
#[derive(Resource)]
struct CrowSpawner {
    timer: Timer,
    count: u64,
}

/// Cooldown between shots.
#[derive(Resource)]
struct FireCooldown {
    timer: Timer,
}

/// Short-lived flash/spark, despawned by `despawn_after`.
#[derive(Component)]
struct Life {
    timer: Timer,
}

const CROW_SPEED: f32 = 150.0;
const BULLET_SPEED: f32 = 750.0;
const MAX_CROWS: usize = 8;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CombatStats::default())
            .insert_resource(CrowSpawner {
                timer: Timer::from_seconds(2.5, TimerMode::Repeating),
                count: 0,
            })
            .insert_resource(FireCooldown {
                timer: Timer::from_seconds(0.22, TimerMode::Once),
            })
            .add_systems(
                Update,
                (
                    spawn_crows,
                    fire_gun,
                    move_crows,
                    move_bullets,
                    flap_crows,
                    bullet_hits,
                    crow_hits_player,
                    despawn_after,
                ),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_crows(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<LevelState>,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut spawner: ResMut<CrowSpawner>,
    player: Query<&Transform, With<Player>>,
    camera: Query<&Transform, (With<Camera>, Without<Player>)>,
    crows: Query<(), With<Crow>>,
) {
    if state.won {
        return;
    }
    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }
    if crows.iter().len() >= MAX_CROWS {
        return;
    }
    let (Ok(p), Ok(cam)) = (player.single(), camera.single()) else {
        return;
    };

    // Alternate sides, deterministic-ish height variety, no rand dependency.
    let side = if spawner.count.is_multiple_of(2) { 1.0 } else { -1.0 };
    spawner.count += 1;
    let y_var = ((spawner.count * 137) % 300) as f32 - 100.0;

    let pos = Vec3::new(cam.translation.x + side * 800.0, p.translation.y + 80.0 + y_var, 3.0);
    let dir = (p.translation.truncate() - pos.truncate()).normalize_or_zero();
    let layout = TextureAtlasLayout::from_grid(UVec2::new(48, 48), 2, 1, None, None);

    commands.spawn((
        Crow {
            vel: dir * CROW_SPEED,
            phase: spawner.count as f32,
        },
        CrowFlap {
            timer: Timer::from_seconds(0.15, TimerMode::Repeating),
        },
        Sprite {
            image: assets.load("Gothicvania/Characters/crow/Spritesheets/crow-fly.png"),
            texture_atlas: Some(TextureAtlas {
                layout: layouts.add(layout),
                index: 0,
            }),
            ..default()
        },
        // Crow art faces left; mirror when flying right.
        Transform {
            translation: pos,
            scale: Vec3::new(if dir.x >= 0.0 { -2.0 } else { 2.0 }, 2.0, 1.0),
            ..default()
        },
    ));
}

fn fire_gun(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    state: Res<LevelState>,
    mut cd: ResMut<FireCooldown>,
    player: Query<(&Transform, &FacingLR), With<Player>>,
) {
    cd.timer.tick(time.delta());
    if state.won {
        return;
    }
    if !input.pressed(KeyCode::KeyJ) || !cd.timer.is_finished() {
        return;
    }
    let Ok((t, facing)) = player.single() else {
        return;
    };
    cd.timer.reset();

    let s = match facing {
        FacingLR::Right => 1.0,
        FacingLR::Left => -1.0,
    };
    let muzzle = t.translation.truncate() + muzzle_offset(*facing);
    // Tracer round.
    commands.spawn((
        Bullet {
            vel: Vec2::new(BULLET_SPEED * s, 0.0),
        },
        Sprite {
            color: Color::srgb(1.0, 0.9, 0.45),
            custom_size: Some(Vec2::new(16.0, 5.0)),
            ..default()
        },
        Transform::from_xyz(muzzle.x, muzzle.y, 4.0),
    ));
    // Muzzle flash.
    commands.spawn((
        Life {
            timer: Timer::from_seconds(0.06, TimerMode::Once),
        },
        Sprite {
            color: Color::srgb(1.0, 0.75, 0.3),
            custom_size: Some(Vec2::new(22.0, 22.0)),
            ..default()
        },
        Transform::from_xyz(muzzle.x, muzzle.y, 4.5),
    ));
}

fn move_crows(
    mut commands: Commands,
    time: Res<Time>,
    mut crows: Query<(Entity, &mut Transform, &Crow), Without<Camera>>,
    camera: Query<&Transform, With<Camera>>,
) {
    let Ok(cam) = camera.single() else { return };
    for (e, mut t, c) in &mut crows {
        let wobble = (time.elapsed_secs() * 3.0 + c.phase).sin() * 35.0;
        t.translation.x += c.vel.x * time.delta_secs();
        t.translation.y += (c.vel.y + wobble) * time.delta_secs();
        // Clean up crows left far behind the camera.
        if (t.translation.x - cam.translation.x).abs() > 1200.0 {
            commands.entity(e).despawn();
        }
    }
}

fn move_bullets(
    mut commands: Commands,
    time: Res<Time>,
    camera: Query<&Transform, With<Camera>>,
    mut bullets: Query<(Entity, &mut Transform, &Bullet), Without<Camera>>,
) {
    let Ok(cam) = camera.single() else { return };
    for (e, mut t, b) in &mut bullets {
        t.translation += b.vel.extend(0.0) * time.delta_secs();
        if (t.translation.x - cam.translation.x).abs() > 950.0 {
            commands.entity(e).despawn();
        }
    }
}

fn flap_crows(time: Res<Time>, mut crows: Query<(&mut CrowFlap, &mut Sprite), With<Crow>>) {
    for (mut flap, mut sprite) in &mut crows {
        flap.timer.tick(time.delta());
        if flap.timer.just_finished()
            && let Some(atlas) = sprite.texture_atlas.as_mut()
        {
            atlas.index = (atlas.index + 1) % 2;
        }
    }
}

#[allow(clippy::type_complexity)]
fn bullet_hits(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    crows: Query<(Entity, &Transform), (With<Crow>, Without<Bullet>)>,
    mut stats: ResMut<CombatStats>,
) {
    for (be, bt) in &bullets {
        for (ce, ct) in &crows {
            let dx = (bt.translation.x - ct.translation.x).abs();
            let dy = (bt.translation.y - ct.translation.y).abs();
            if dx < 36.0 && dy < 26.0 {
                commands.entity(be).despawn();
                commands.entity(ce).despawn();
                stats.kills += 1;
                // Hit spark.
                commands.spawn((
                    Life {
                        timer: Timer::from_seconds(0.15, TimerMode::Once),
                    },
                    Sprite {
                        color: Color::srgb(1.0, 0.4, 0.3),
                        custom_size: Some(Vec2::new(30.0, 30.0)),
                        ..default()
                    },
                    Transform::from_xyz(ct.translation.x, ct.translation.y, 4.5),
                ));
                break;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn crow_hits_player(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    crows: Query<(Entity, &Transform), (With<Crow>, Without<Player>)>,
    mut stats: ResMut<CombatStats>,
) {
    let Ok(p) = player.single() else { return };
    for (ce, ct) in &crows {
        let dx = (p.translation.x - ct.translation.x).abs();
        let dy = (p.translation.y - ct.translation.y).abs();
        if dx < 48.0 && dy < 62.0 {
            commands.entity(ce).despawn();
            stats.hits += 1;
        }
    }
}

fn despawn_after(mut commands: Commands, time: Res<Time>, mut query: Query<(Entity, &mut Life)>) {
    for (e, mut life) in &mut query {
        life.timer.tick(time.delta());
        if life.timer.is_finished() {
            commands.entity(e).despawn();
        }
    }
}
