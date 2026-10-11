use bevy::prelude::*;

use crate::level::{LevelState, SpawnPoint};
use crate::player::{muzzle_offset, FacingLR, Health, Player, Velocity};

/// Kills by gunfire + times an eye reached the player.
#[derive(Resource, Default)]
pub struct CombatStats {
    pub kills: u32,
    pub hits: u32,
}

/// A flying eye demon homing in on the player.
#[derive(Component)]
struct Eye {
    vel: Vec2,
    phase: f32,
}

/// Bullet in flight.
#[derive(Component)]
struct Bullet {
    vel: Vec2,
}

/// Fired every few seconds while the level runs.
#[derive(Resource)]
struct EyeSpawner {
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

const EYE_SPEED: f32 = 150.0;
const BULLET_SPEED: f32 = 750.0;
const EYE_DAMAGE: f32 = 9.0;
const MAX_EYES: usize = 8;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CombatStats::default())
            .insert_resource(EyeSpawner {
                timer: Timer::from_seconds(2.5, TimerMode::Repeating),
                count: 0,
            })
            .insert_resource(FireCooldown {
                timer: Timer::from_seconds(0.22, TimerMode::Once),
            })
            .add_systems(
                Update,
                (
                    spawn_eyes,
                    fire_gun,
                    move_eyes,
                    move_bullets,
                    face_player,
                    bullet_hits,
                    eye_hits_player,
                    despawn_after,
                ),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_eyes(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<LevelState>,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut spawner: ResMut<EyeSpawner>,
    player: Query<&Transform, With<Player>>,
    camera: Query<&Transform, (With<Camera>, Without<Player>)>,
    eyes: Query<(), With<Eye>>,
) {
    if state.won {
        return;
    }
    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }
    if eyes.iter().len() >= MAX_EYES {
        return;
    }
    let (Ok(p), Ok(cam)) = (player.single(), camera.single()) else {
        return;
    };

    // Alternate sides, deterministic-ish height variety, no rand dependency.
    let side = if spawner.count.is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };
    spawner.count += 1;
    let y_var = ((spawner.count * 137) % 300) as f32 - 100.0;

    let pos = Vec3::new(
        cam.translation.x + side * 800.0,
        p.translation.y + 80.0 + y_var,
        3.0,
    );
    let dir = (p.translation.truncate() - pos.truncate()).normalize_or_zero();
    let layout = TextureAtlasLayout::from_grid(UVec2::new(48, 48), 8, 1, None, None);

    commands.spawn((
        Eye {
            vel: dir * EYE_SPEED,
            phase: spawner.count as f32,
        },
        Sprite {
            image: assets.load("Gothicvania/Characters/flying-eye-demon/Spritesheet.png"),
            texture_atlas: Some(TextureAtlas {
                layout: layouts.add(layout),
                index: 0,
            }),
            ..default()
        },
        // Eye art is symmetric; no mirroring needed.
        Transform {
            translation: pos,
            scale: Vec3::splat(2.0),
            ..default()
        },
    ));
}

fn fire_gun(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    state: Res<LevelState>,
    audio: Res<AssetServer>,
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

    commands.spawn(AudioPlayer::new(audio.load("sfx/shoot.ogg")));
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

#[allow(clippy::type_complexity)]
fn move_eyes(
    time: Res<Time>,
    state: Res<LevelState>,
    player: Query<&Transform, With<Player>>,
    mut eyes: Query<(&mut Transform, &mut Eye), (Without<Camera>, Without<Player>)>,
) {
    let dt = time.delta_secs();
    let target = player.single().ok().map(|t| t.translation.truncate());
    for (mut t, mut e) in &mut eyes {
        // Steer toward the player so eyes genuinely chase instead of
        // flying a fixed line. Frozen once the level is won.
        if !state.won && let Some(tp) = target {
            let to = tp - t.translation.truncate();
            if to.length_squared() > 1.0 {
                let desired = to.normalize() * EYE_SPEED;
                let k = 1.0 - (-2.0 * dt).exp();
                e.vel = e.vel.lerp(desired, k);
            }
        }
        let wobble = (time.elapsed_secs() * 3.0 + e.phase).sin() * 35.0;
        t.translation.x += e.vel.x * dt;
        t.translation.y += (e.vel.y + wobble) * dt;
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

/// Pick the turn-frame that looks at the player.
/// The sheet's profile frames face right natively (eye on the right),
/// so they are flipped for players on the left. Head-on chomps need
/// no flip.
#[allow(clippy::type_complexity)]
fn face_player(
    time: Res<Time>,
    player: Query<&Transform, With<Player>>,
    mut eyes: Query<(&mut Transform, &Eye, &mut Sprite), (With<Eye>, Without<Player>)>,
) {
    let Ok(p) = player.single() else { return };
    for (mut t, e, mut sprite) in &mut eyes {
        let dx = p.translation.x - t.translation.x;
        let (index, flipped) = if dx < -140.0 {
            (0, true)
        } else if dx < -50.0 {
            (1, true)
        } else if dx < -15.0 {
            (2, true)
        } else if dx <= 15.0 {
            // Head-on: alternate the two mouth frames for a chomp.
            let chomp = (time.elapsed_secs() * 2.0 + e.phase) % 2.0 < 1.0;
            (if chomp { 3 } else { 4 }, false)
        } else if dx <= 50.0 {
            (2, false)
        } else if dx <= 140.0 {
            (1, false)
        } else {
            (0, false)
        };
        t.scale.x = if flipped { -2.0 } else { 2.0 };
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
    }
}

#[allow(clippy::type_complexity)]
fn bullet_hits(
    mut commands: Commands,
    audio: Res<AssetServer>,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    eyes: Query<(Entity, &Transform), (With<Eye>, Without<Bullet>)>,
    mut stats: ResMut<CombatStats>,
) {
    for (be, bt) in &bullets {
        for (ee, et) in &eyes {
            let dx = (bt.translation.x - et.translation.x).abs();
            let dy = (bt.translation.y - et.translation.y).abs();
            if dx < 36.0 && dy < 26.0 {
                commands.entity(be).despawn();
                commands.entity(ee).despawn();
                stats.kills += 1;
                commands.spawn(AudioPlayer::new(audio.load("sfx/crow_die.ogg")));
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
                    Transform::from_xyz(et.translation.x, et.translation.y, 4.5),
                ));
                break;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn eye_hits_player(
    mut commands: Commands,
    time: Res<Time>,
    audio: Res<AssetServer>,
    spawn: Res<SpawnPoint>,
    mut state: ResMut<LevelState>,
    mut player: Query<(&mut Transform, &mut Velocity, &mut Health), With<Player>>,
    eyes: Query<(Entity, &Transform), (With<Eye>, Without<Player>)>,
    mut stats: ResMut<CombatStats>,
) {
    let Ok((mut t, mut vel, mut health)) = player.single_mut() else {
        return;
    };
    health.invuln.tick(time.delta());
    for (ee, et) in &eyes {
        let dx = (t.translation.x - et.translation.x).abs();
        let dy = (t.translation.y - et.translation.y).abs();
        if dx >= 48.0 || dy >= 62.0 {
            continue;
        }
        commands.entity(ee).despawn();
        // I-frames: one damaging hit per second at most.
        if !health.invuln.is_finished() {
            continue;
        }
        health.invuln.reset();
        health.hp -= EYE_DAMAGE;
        stats.hits += 1;
        commands.spawn(AudioPlayer::new(audio.load("sfx/hurt.ogg")));
        if health.hp <= 0.0 {
            // Killed: respawn fresh like a fall.
            t.translation = spawn.0;
            vel.0 = Vec2::ZERO;
            *health = Health::full();
            state.deaths += 1;
            break;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LevelPlugin;
    use crate::player::{PlayerPlugin, Velocity};
    use bevy::audio::Decodable;

    /// Every SFX must fully decode with the enabled backend.
    /// (A previous WAV set panicked at runtime with `UnrecognizedFormat`.)
    #[test]
    fn sfx_files_decode() {
        for name in ["shoot", "crow_die", "hurt", "jump", "win"] {
            let path = format!(
                "{}/src/assets/sfx/{}.ogg",
                env!("CARGO_MANIFEST_DIR"),
                name
            );
            let bytes = std::fs::read(&path).expect("sfx file exists");
            let source = AudioSource { bytes: bytes.into() };
            let samples = source.decoder().count();
            assert!(samples > 0, "{name} decodes to audio");
        }
        // Background music track too.
        let music = format!(
            "{}/src/assets/2023 _ Clement Panchout - Unrecord.ogg",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&music).expect("music file exists");
        let source = AudioSource { bytes: bytes.into() };
        assert!(source.decoder().count() > 0, "music decodes to audio");
    }

    #[test]
    fn eye_touch_damages_player_by_nine() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugins(bevy::input::InputPlugin);
        app.init_asset::<Image>();
        app.init_asset::<AudioSource>();
        app.init_asset::<TextureAtlasLayout>();
        app.add_plugins((LevelPlugin, PlayerPlugin, CombatPlugin));

        // Let Startup spawn the player, platforms, spawners.
        for _ in 0..10 {
            app.update();
        }

        let player_pos = app
            .world_mut()
            .query_filtered::<&Transform, With<Player>>()
            .single(app.world())
            .expect("player spawned")
            .translation;
        let hp_before = app
            .world_mut()
            .query_filtered::<&Health, With<Player>>()
            .single(app.world())
            .expect("player health")
            .hp;

        // Drop an eye exactly on the player.
        app.world_mut().spawn((
            Eye {
                vel: Vec2::ZERO,
                phase: 0.0,
            },
            Transform::from_translation(player_pos),
        ));
        app.update();

        let hp_after = app
            .world_mut()
            .query_filtered::<&Health, With<Player>>()
            .single(app.world())
            .expect("player health")
            .hp;
        let stats = app.world().resource::<CombatStats>();
        assert_eq!(hp_before - hp_after, EYE_DAMAGE, "eye hit is -9% HP");
        assert_eq!(stats.hits, 1);
        // Velocity access keeps the import used on all platforms.
        let _ = app
            .world_mut()
            .query_filtered::<&Velocity, With<Player>>()
            .single(app.world());

        // Death branch: nearly-dead player takes another hit -> respawn fresh.
        let spawn_pos = app.world().resource::<crate::level::SpawnPoint>().0;
        app.world_mut()
            .query_filtered::<&mut Health, With<Player>>()
            .single_mut(app.world_mut())
            .expect("player health")
            .hp = 5.0;
        let ppos = app
            .world_mut()
            .query_filtered::<&Transform, With<Player>>()
            .single(app.world())
            .expect("player pos")
            .translation;
        // Clear i-frames by exhausting the timer via many ticks is slow;
        // instead reset invuln to finished directly.
        app.world_mut()
            .query_filtered::<&mut Health, With<Player>>()
            .single_mut(app.world_mut())
            .expect("player health")
            .invuln
            .finish();
        app.world_mut().spawn((
            Eye {
                vel: Vec2::ZERO,
                phase: 1.0,
            },
            Transform::from_translation(ppos),
        ));
        app.update();

        let health = app
            .world_mut()
            .query_filtered::<&Health, With<Player>>()
            .single(app.world())
            .expect("player health");
        assert_eq!(health.hp, Health::MAX, "death restores full HP");
        let pos = app
            .world_mut()
            .query_filtered::<&Transform, With<Player>>()
            .single(app.world())
            .expect("player pos")
            .translation;
        assert!(
            pos.distance(spawn_pos) < 5.0,
            "death respawns at spawn point (got {pos})"
        );
        assert_eq!(app.world().resource::<LevelState>().deaths, 1);
    }
}
