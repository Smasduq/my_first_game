use bevy::prelude::*;

use crate::combat::{Bullet, Eye, Impact, Life};
use crate::menu::{GameState, SfxEnabled};
use crate::player::{Health, Player, Velocity};

/// Solid ground. `size` is full width/height, centered on the Transform.
#[derive(Component)]
pub struct Platform {
    pub size: Vec2,
}

/// Final finish flag at the end of the last segment.
#[derive(Component)]
pub struct Goal;

/// Checkpoint flag at the start of segments 2..=TOTAL_CHECKPOINTS.
/// Touching one makes it the respawn point. Holds the segment number.
#[derive(Component)]
pub struct CheckpointFlag(pub u32);

/// Where the player respawns: the last checkpoint reached.
#[derive(Resource)]
pub struct SpawnPoint(pub Vec3);

/// Marker on course geometry (platforms, lips, seams, flags, trees, lanterns).
#[derive(Component)]
pub struct LevelGeometry;

/// Total checkpoints in the course. Checkpoint 1 is the start line;
/// the last segment ends with the ESCAPE flag.
pub const TOTAL_CHECKPOINTS: u32 = 10;

/// Flat gap between the end of one segment and the start of the next.
pub const TRANSITION_GAP: f32 = 120.0;

/// Deaths + win state + highest checkpoint reached (1-based).
#[derive(Resource)]
pub struct LevelState {
    pub deaths: u32,
    pub won: bool,
    pub current: u32,
}

impl Default for LevelState {
    fn default() -> Self {
        Self {
            deaths: 0,
            won: false,
            current: 1,
        }
    }
}

/// Per-checkpoint enemy pressure. Scales monotonically down the course.
#[derive(Debug, Clone, Copy)]
pub struct Difficulty {
    /// Seconds between eye spawns.
    pub eye_interval: f32,
    /// Cap on live eyes.
    pub max_eyes: usize,
    /// Eye homing speed.
    pub eye_speed: f32,
    /// HP lost per eye touch.
    pub eye_damage: f32,
}

/// Checkpoint 1 is calm, checkpoint 10 is relentless. Linear interpolation.
pub fn difficulty_for(checkpoint: u32) -> Difficulty {
    let l = checkpoint.clamp(1, TOTAL_CHECKPOINTS) as f32 - 1.0;
    let t = l / (TOTAL_CHECKPOINTS as f32 - 1.0);
    Difficulty {
        eye_interval: 4.5 - 3.3 * t,
        max_eyes: (2.0 + 8.0 * t).round() as usize,
        eye_speed: 110.0 + 110.0 * t,
        eye_damage: 7.0 + 9.0 * t,
    }
}

/// One repeating slice of a parallax background layer.
/// `slot` is this copy's index; copies tile seamlessly around the camera.
#[derive(Component)]
struct BgTile {
    factor: f32,
    slot: usize,
    count: usize,
    tile_w: f32,
}

/// Translucent drifting fog puff.
#[derive(Component)]
struct Fog {
    speed: f32,
}

#[derive(Component)]
struct LanternGlow {
    phase: f32,
}

pub const KILL_Y: f32 = -500.0;

// ---------------------------------------------------------------------------
// Course segments: platforms as (x_min, x_max, top_y) in segment-local
// coordinates. Segments are laid end-to-end with TRANSITION_GAP between
// them, easiest first, hardest last. Jump physics (330 run, 950 jump vel,
// 2600 gravity) clears ~240px flat and ~173px up, so flat gaps top out at
// 210 and climbs keep gaps <= 150.
// ---------------------------------------------------------------------------

const L1: &[(f32, f32, f32)] = &[
    (-350.0, 350.0, 0.0),
    (450.0, 1050.0, 0.0),
    (1150.0, 1750.0, 0.0),
];

const L2: &[(f32, f32, f32)] = &[
    (-350.0, 300.0, 0.0),
    (420.0, 970.0, 0.0),
    (1090.0, 1590.0, 30.0),
    (1710.0, 2310.0, 30.0),
];

const L3: &[(f32, f32, f32)] = &[
    (-350.0, 250.0, 0.0),
    (380.0, 930.0, 0.0),
    (1060.0, 1510.0, 50.0),
    (1640.0, 2240.0, 20.0),
];

const L4: &[(f32, f32, f32)] = &[
    (-350.0, 200.0, 0.0),
    (340.0, 840.0, 0.0),
    (980.0, 1430.0, 60.0),
    (1570.0, 2020.0, 60.0),
    (2160.0, 2710.0, 20.0),
];

const L5: &[(f32, f32, f32)] = &[
    (-350.0, 150.0, 0.0),
    (300.0, 750.0, 0.0),
    (900.0, 1300.0, 70.0),
    (1450.0, 1850.0, 70.0),
    (2000.0, 2500.0, 30.0),
];

const L6: &[(f32, f32, f32)] = &[
    (-350.0, 150.0, 0.0),
    (310.0, 760.0, 0.0),
    (920.0, 1320.0, 80.0),
    (1480.0, 1880.0, 80.0),
    (2040.0, 2440.0, 40.0),
    (2600.0, 3050.0, 40.0),
];

const L7: &[(f32, f32, f32)] = &[
    (-350.0, 100.0, 0.0),
    (275.0, 675.0, 0.0),
    (825.0, 1250.0, 90.0),
    (1425.0, 1825.0, 90.0),
    (2000.0, 2400.0, 40.0),
    (2575.0, 3025.0, 80.0),
];

const L8: &[(f32, f32, f32)] = &[
    (-350.0, 50.0, 0.0),
    (235.0, 635.0, 0.0),
    (785.0, 1135.0, 60.0),
    (1285.0, 1685.0, 60.0),
    (1870.0, 2270.0, 10.0),
    (2420.0, 2770.0, 70.0),
    (2920.0, 3320.0, 70.0),
];

const L9: &[(f32, f32, f32)] = &[
    (-350.0, 0.0, 0.0),
    (195.0, 545.0, 0.0),
    (695.0, 1045.0, 70.0),
    (1195.0, 1545.0, 70.0),
    (1740.0, 2090.0, 20.0),
    (2240.0, 2590.0, 90.0),
    (2740.0, 3140.0, 90.0),
];

const L10: &[(f32, f32, f32)] = &[
    (-350.0, -50.0, 0.0),
    (160.0, 460.0, 0.0),
    (610.0, 910.0, 80.0),
    (1060.0, 1360.0, 80.0),
    (1570.0, 1870.0, 0.0),
    (2020.0, 2320.0, 100.0),
    (2470.0, 2770.0, 100.0),
    (2980.0, 3330.0, 40.0),
];

/// Segment-local platforms for a 1-based segment number (clamped).
pub fn platforms_for(segment: u32) -> &'static [(f32, f32, f32)] {
    match segment.clamp(1, TOTAL_CHECKPOINTS) {
        1 => L1,
        2 => L2,
        3 => L3,
        4 => L4,
        5 => L5,
        6 => L6,
        7 => L7,
        8 => L8,
        9 => L9,
        _ => L10,
    }
}

/// World x-offset for a segment so segments tile end-to-end:
/// each segment starts TRANSITION_GAP past the previous segment's end.
pub fn segment_offset(segment: u32) -> f32 {
    let segment = segment.clamp(1, TOTAL_CHECKPOINTS);
    let mut offset = 0.0;
    let mut prev_global_end = 0.0;
    for seg in 1..=segment {
        let plats = platforms_for(seg);
        let local_start = plats[0].0;
        let local_end = plats[plats.len() - 1].1;
        if seg == 1 {
            offset = 0.0;
        } else {
            offset = prev_global_end + TRANSITION_GAP - local_start;
        }
        prev_global_end = offset + local_end;
    }
    offset
}

/// Respawn point for a checkpoint: above the start of its segment.
pub fn checkpoint_spawn(checkpoint: u32) -> Vec3 {
    let checkpoint = checkpoint.clamp(1, TOTAL_CHECKPOINTS);
    let off = segment_offset(checkpoint);
    let (x_min, _, top) = platforms_for(checkpoint)[0];
    Vec3::new(off + x_min + 100.0, top + 150.0, 0.0)
}

/// Flag position for checkpoints 2..=TOTAL (banner sits near the
/// segment start, just past the transition gap). Returns (x, top).
pub fn checkpoint_flag_pos(checkpoint: u32) -> Option<(f32, f32)> {
    let checkpoint = checkpoint.clamp(1, TOTAL_CHECKPOINTS);
    if checkpoint < 2 {
        return None;
    }
    let off = segment_offset(checkpoint);
    let (x_min, _, top) = platforms_for(checkpoint)[0];
    Some((off + x_min + 60.0, top))
}

/// Final ESCAPE flag at the end of the last segment. Returns (x, top).
pub fn final_goal_pos() -> (f32, f32) {
    let off = segment_offset(TOTAL_CHECKPOINTS);
    let plats = platforms_for(TOTAL_CHECKPOINTS);
    let (_, x_max, top) = plats[plats.len() - 1];
    (off + x_max - 80.0, top)
}

/// (asset path, pixel width, parallax factor, z).
/// All Mountain Dusk A layers are 240px tall; drawn at 4x with nearest filter.
const BG_LAYERS: [(&str, f32, f32, f32); 6] = [
    (
        "Gothicvania/Environments/Mountain Dusk/version A/Layers/sky.png",
        320.0,
        0.0,
        -100.0,
    ),
    (
        "Gothicvania/Environments/Mountain Dusk/version A/Layers/far-clouds.png",
        128.0,
        0.1,
        -90.0,
    ),
    (
        "Gothicvania/Environments/Mountain Dusk/version A/Layers/far-mountains.png",
        160.0,
        0.2,
        -80.0,
    ),
    (
        "Gothicvania/Environments/Mountain Dusk/version A/Layers/mountains.png",
        320.0,
        0.4,
        -70.0,
    ),
    (
        "Gothicvania/Environments/Mountain Dusk/version A/Layers/near-clouds.png",
        144.0,
        0.5,
        -60.0,
    ),
    (
        "Gothicvania/Environments/Mountain Dusk/version A/Layers/trees.png",
        240.0,
        0.7,
        -50.0,
    ),
];

const BG_SCALE: f32 = 4.0;
/// Half-width of the camera-relative window tiles wrap inside.
const BG_MARGIN: f32 = 900.0;

/// Pending checkpoint banner vs claimed checkpoint banner.
const FLAG_PENDING: Color = Color::srgb(0.91, 0.62, 0.26);
const FLAG_DONE: Color = Color::srgb(0.35, 0.75, 0.40);

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpawnPoint(checkpoint_spawn(1)))
            .insert_resource(LevelState::default())
            .add_systems(Startup, (build_course, spawn_background, spawn_fog))
            .add_systems(
                Update,
                (
                    scroll_background,
                    drift_fog,
                    flicker_lanterns,
                    activate_checkpoints,
                    tint_checkpoint_flags,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            // Ambient systems also run while paused so the menu backdrop lives.
            .add_systems(
                Update,
                (scroll_background, drift_fog, flicker_lanterns)
                    .run_if(in_state(GameState::Menu)),
            );
    }
}

/// Spawn the whole course once: all segments end-to-end, checkpoint
/// flags at segments 2..=10, ESCAPE flag at the very end.
fn build_course(mut commands: Commands) {
    for seg in 1..=TOTAL_CHECKPOINTS {
        spawn_platforms_for(&mut commands, seg);
        spawn_trees_for(&mut commands, seg);
        spawn_lanterns_for(&mut commands, seg);
        if seg >= 2 {
            spawn_checkpoint_flag_for(&mut commands, seg);
        }
    }
    spawn_final_goal_for(&mut commands);
}

fn spawn_platforms_for(commands: &mut Commands, segment: u32) {
    // Mountain Dusk palette: near-black purple rock, sunlit red-orange lip.
    let ground = Color::srgb(0.10, 0.06, 0.15);
    let under_lip = Color::srgb(0.05, 0.03, 0.08);
    let edge = Color::srgb(0.52, 0.21, 0.19);
    let off = segment_offset(segment);

    for (x_min, x_max, top) in platforms_for(segment) {
        let (x_min, x_max, top) = (off + *x_min, off + *x_max, *top);
        let w = x_max - x_min;
        let cx = (x_min + x_max) * 0.5;
        let h = 90.0;
        commands.spawn((
            LevelGeometry,
            Platform {
                size: Vec2::new(w, h),
            },
            Sprite {
                color: ground,
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform::from_xyz(cx, top - h * 0.5, 0.0),
        ));
        // Sunlit top lip plus a dark under-shadow for pixel-art depth.
        commands.spawn((
            LevelGeometry,
            Sprite {
                color: edge,
                custom_size: Some(Vec2::new(w, 8.0)),
                ..default()
            },
            Transform::from_xyz(cx, top - 4.0, 0.5),
        ));
        commands.spawn((
            LevelGeometry,
            Sprite {
                color: under_lip,
                custom_size: Some(Vec2::new(w, 6.0)),
                ..default()
            },
            Transform::from_xyz(cx, top - 11.0, 0.5),
        ));

        let mut seam_x = x_min + 56.0;
        let mut seam_index = 0;
        while seam_x < x_max - 32.0 {
            let seam_height = 24.0 + (seam_index % 3) as f32 * 8.0;
            commands.spawn((
                LevelGeometry,
                Sprite {
                    color: Color::srgb(0.13, 0.08, 0.17),
                    custom_size: Some(Vec2::new(3.0, seam_height)),
                    ..default()
                },
                Transform::from_xyz(seam_x, top - 30.0, 0.6),
            ));
            commands.spawn((
                LevelGeometry,
                Sprite {
                    color: Color::srgb(0.16, 0.09, 0.17),
                    custom_size: Some(Vec2::new(18.0, 2.0)),
                    ..default()
                },
                Transform::from_xyz(seam_x + 9.0, top - 30.0 - seam_height * 0.5, 0.6),
            ));
            seam_x += 96.0;
            seam_index += 1;
        }
    }
}

fn spawn_checkpoint_flag_for(commands: &mut Commands, checkpoint: u32) {
    let Some((flag_x, top)) = checkpoint_flag_pos(checkpoint) else {
        return;
    };
    // Pole.
    commands.spawn((
        LevelGeometry,
        Sprite {
            color: Color::srgb(0.6, 0.58, 0.55),
            custom_size: Some(Vec2::new(8.0, 170.0)),
            ..default()
        },
        Transform::from_xyz(flag_x, top + 85.0, 1.0),
    ));
    // Banner, tinted by claimed state.
    commands.spawn((
        LevelGeometry,
        CheckpointFlag(checkpoint),
        Sprite {
            color: FLAG_PENDING,
            custom_size: Some(Vec2::new(70.0, 44.0)),
            ..default()
        },
        Transform::from_xyz(flag_x + 39.0, top + 148.0, 1.0),
    ));
    commands.spawn((
        LevelGeometry,
        CheckpointFlag(checkpoint),
        Text2d::new(checkpoint.to_string()),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(0.1, 0.08, 0.1)),
        Transform::from_xyz(flag_x + 39.0, top + 148.0, 2.0),
    ));
}

fn spawn_final_goal_for(commands: &mut Commands) {
    let (goal_x, top) = final_goal_pos();
    // Pole.
    commands.spawn((
        LevelGeometry,
        Sprite {
            color: Color::srgb(0.6, 0.58, 0.55),
            custom_size: Some(Vec2::new(8.0, 170.0)),
            ..default()
        },
        Transform::from_xyz(goal_x, top + 85.0, 1.0),
    ));
    // Banner.
    commands.spawn((
        LevelGeometry,
        Goal,
        Sprite {
            color: Color::srgb(0.55, 0.12, 0.14),
            custom_size: Some(Vec2::new(70.0, 44.0)),
            ..default()
        },
        Transform::from_xyz(goal_x + 39.0, top + 148.0, 1.0),
    ));
    commands.spawn((
        LevelGeometry,
        Goal,
        Text2d::new("ESCAPE"),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.75, 0.75)),
        Transform::from_xyz(goal_x + 39.0, top + 148.0, 2.0),
    ));
}

fn flag_hit(px: f32, py: f32, gx: f32, gy: f32) -> bool {
    // Generous banner hitbox.
    (px - gx).abs() < 60.0 && (py - gy).abs() < 110.0
}

/// Touching a checkpoint flag claims it: respawn moves there, HP
/// refills, and pursuers are cleared — the run continues uninterrupted.
/// Touching the ESCAPE flag wins the run.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn activate_checkpoints(
    mut commands: Commands,
    audio: Res<AssetServer>,
    sfx: Res<SfxEnabled>,
    mut player: Query<(&mut Transform, &mut Velocity, &mut Health), With<Player>>,
    flags: Query<(&Transform, &CheckpointFlag), Without<Player>>,
    goals: Query<&Transform, (With<Goal>, Without<Player>)>,
    mut state: ResMut<LevelState>,
    mut spawn: ResMut<SpawnPoint>,
    eyes: Query<Entity, With<Eye>>,
    bullets: Query<Entity, With<Bullet>>,
    fx: Query<Entity, Or<(With<Life>, With<Impact>)>>,
) {
    if state.won {
        return;
    }
    let (px, py) = {
        let Ok((p, _, _)) = player.single() else {
            return;
        };
        (p.translation.x, p.translation.y)
    };

    for g in &goals {
        if flag_hit(px, py, g.translation.x, g.translation.y) {
            state.won = true;
            if sfx.0 {
                commands.spawn(AudioPlayer::new(
                    audio.load("10_Battle_SFX/55_Encounter_02.ogg"),
                ));
            }
            return;
        }
    }

    // Claim the highest newly-touched checkpoint (normally the next one).
    let mut best: Option<u32> = None;
    for (t, flag) in &flags {
        if flag.0 > state.current
            && flag_hit(px, py, t.translation.x, t.translation.y)
            && best.is_none_or(|b| flag.0 > b)
        {
            best = Some(flag.0);
        }
    }
    let Some(id) = best else { return };
    state.current = id;
    spawn.0 = checkpoint_spawn(id);
    // Checkpoint breather: full HP, pursuers cleared, momentum kept.
    for e in eyes.iter().chain(bullets.iter()) {
        commands.entity(e).despawn();
    }
    for e in &fx {
        commands.entity(e).despawn();
    }
    if let Ok((_, _, mut h)) = player.single_mut() {
        *h = Health::full();
    }
    if sfx.0 {
        commands.spawn(AudioPlayer::new(
            audio.load("10_Battle_SFX/55_Encounter_02.ogg"),
        ));
    }
}

/// Claimed checkpoint banners turn green; upcoming ones stay amber.
fn tint_checkpoint_flags(
    state: Res<LevelState>,
    mut banners: Query<(&CheckpointFlag, &mut Sprite)>,
) {
    for (flag, mut sprite) in &mut banners {
        sprite.color = if flag.0 <= state.current {
            FLAG_DONE
        } else {
            FLAG_PENDING
        };
    }
}

/// Real art background: Mountain Dusk A layers, tiled around the camera.
fn spawn_background(mut commands: Commands, assets: Res<AssetServer>) {
    for (path, px_w, factor, z) in BG_LAYERS {
        let tile_w = px_w * BG_SCALE;
        // Enough copies to cover the wrap window (>= 1600 wide) plus one spare.
        let count = (1600.0 / tile_w).ceil() as usize + 1;
        let image: Handle<Image> = assets.load(path);
        for slot in 0..count {
            commands.spawn((
                BgTile {
                    factor,
                    slot,
                    count,
                    tile_w,
                },
                Sprite {
                    image: image.clone(),
                    ..default()
                },
                Transform {
                    translation: Vec3::new(slot as f32 * tile_w, 0.0, z),
                    scale: Vec3::splat(BG_SCALE),
                    ..default()
                },
            ));
        }
    }
}

/// Wrap each layer's copies around the camera so they tile forever.
/// Lower factor = drifts slower = feels farther away.
fn scroll_background(
    cam: Query<&Transform, With<Camera>>,
    mut tiles: Query<(&mut Transform, &BgTile), Without<Camera>>,
) {
    let Ok(cam) = cam.single() else { return };
    for (mut t, b) in &mut tiles {
        let total = b.count as f32 * b.tile_w;
        let anchor = cam.translation.x * b.factor;
        let rel =
            (b.slot as f32 * b.tile_w + anchor - (cam.translation.x - BG_MARGIN)).rem_euclid(total);
        t.translation.x = cam.translation.x - BG_MARGIN + rel;
        // Keep the art horizon registered across layers, just below the action.
        t.translation.y = cam.translation.y - 60.0;
    }
}

/// Flag/banner x positions to keep clear of decor within a segment.
fn decor_avoid_x(segment: u32) -> Vec<f32> {
    let mut avoid = Vec::new();
    if let Some((fx, _)) = checkpoint_flag_pos(segment) {
        avoid.push(fx + 39.0);
    }
    if segment == TOTAL_CHECKPOINTS {
        avoid.push(final_goal_pos().0 + 39.0);
    }
    avoid
}

/// Black dead-tree silhouettes derived from the segment's platforms
/// (foreground depth over the art).
fn spawn_trees_for(commands: &mut Commands, segment: u32) {
    let black = Color::srgb(0.008, 0.008, 0.015);
    let off = segment_offset(segment);
    let plats = platforms_for(segment);
    let avoid = decor_avoid_x(segment);
    // One or two trees per platform depending on width.
    let mut i = 0;
    for (x_min, x_max, top) in plats.iter().copied() {
        let (x_min, x_max) = (off + x_min, off + x_max);
        let w = x_max - x_min;
        let mut xs = vec![(x_min + x_max) * 0.5 - 60.0];
        if w > 450.0 {
            xs.push(x_min + w * 0.75);
        }
        for x in xs {
            if avoid.iter().any(|a| (x - a).abs() < 90.0) {
                continue;
            }
            let h_var = 110.0 + (i as f32 % 3.0) * 25.0;
            // Trunk.
            commands.spawn((
                LevelGeometry,
                Sprite {
                    color: black,
                    custom_size: Some(Vec2::new(12.0, h_var)),
                    ..default()
                },
                Transform::from_xyz(x, top + h_var * 0.5, -5.0),
            ));
            // Two branches.
            for (side, ang) in [(-1.0, 0.6), (1.0, -0.55)] {
                commands.spawn((
                    LevelGeometry,
                    Sprite {
                        color: black,
                        custom_size: Some(Vec2::new(8.0, 55.0)),
                        ..default()
                    },
                    Transform {
                        translation: Vec3::new(x + side * 16.0, top + h_var - 20.0, -5.0),
                        rotation: Quat::from_rotation_z(ang * side),
                        ..default()
                    },
                ));
            }
            i += 1;
        }
    }
}

fn spawn_fog(mut commands: Commands) {
    let fog = Color::srgba(0.45, 0.45, 0.55, 0.07);
    let xs = [
        -500.0, -200.0, 100.0, 420.0, 700.0, 1000.0, 1300.0, 1600.0, 1900.0, 2200.0,
        2500.0, 2800.0, 3100.0,
    ];
    for (i, x) in xs.iter().enumerate() {
        commands.spawn((
            Fog {
                speed: 12.0 + (i as f32 % 4.0) * 6.0,
            },
            Sprite {
                color: fog,
                custom_size: Some(Vec2::new(320.0, 70.0)),
                ..default()
            },
            Transform::from_xyz(*x, -40.0 + (i as f32 % 3.0) * 70.0, 5.0),
        ));
    }
}

fn spawn_lanterns_for(commands: &mut Commands, segment: u32) {
    let off = segment_offset(segment);
    let plats = platforms_for(segment);
    let avoid = decor_avoid_x(segment);
    // One lantern near the middle of each platform.
    for (i, (x_min, x_max, top)) in plats.iter().copied().enumerate() {
        let x = off + (x_min + x_max) * 0.5;
        if avoid.iter().any(|a| (x - a).abs() < 60.0) {
            continue;
        }
        let metal = Color::srgb(0.11, 0.08, 0.12);
        commands.spawn((
            LevelGeometry,
            Sprite {
                color: metal,
                custom_size: Some(Vec2::new(5.0, 30.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 15.0, 2.0),
        ));
        commands.spawn((
            LevelGeometry,
            Sprite {
                color: metal,
                custom_size: Some(Vec2::new(18.0, 4.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 29.0, 2.0),
        ));
        commands.spawn((
            LevelGeometry,
            LanternGlow {
                phase: (segment as f32 * 10.0 + i as f32) * 1.7,
            },
            Sprite {
                color: Color::srgba(0.92, 0.49, 0.15, 0.14),
                custom_size: Some(Vec2::new(30.0, 34.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 38.0, 2.1),
        ));
        commands.spawn((
            LevelGeometry,
            Sprite {
                color: Color::srgb(0.96, 0.61, 0.24),
                custom_size: Some(Vec2::new(9.0, 14.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 38.0, 2.2),
        ));
        commands.spawn((
            LevelGeometry,
            Sprite {
                color: Color::srgb(0.21, 0.12, 0.15),
                custom_size: Some(Vec2::new(15.0, 3.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 47.0, 2.3),
        ));
    }
}

fn flicker_lanterns(time: Res<Time>, mut lanterns: Query<(&LanternGlow, &mut Sprite)>) {
    for (glow, mut sprite) in &mut lanterns {
        let flicker = (time.elapsed_secs() * 5.0 + glow.phase).sin() * 0.035;
        sprite.color = Color::srgba(0.92, 0.49, 0.15, 0.14 + flicker);
    }
}

fn drift_fog(
    time: Res<Time>,
    cam: Query<&Transform, (With<Camera>, Without<Fog>)>,
    mut fogs: Query<(&mut Transform, &Fog), Without<Camera>>,
) {
    let Ok(cam) = cam.single() else { return };
    for (mut t, f) in &mut fogs {
        t.translation.x -= f.speed * time.delta_secs();
        if t.translation.x < cam.translation.x - 750.0 {
            t.translation.x += 1500.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_checkpoints_exist_in_order() {
        assert_eq!(TOTAL_CHECKPOINTS, 10);
        let mut prev_spawn = f32::NEG_INFINITY;
        for seg in 1..=TOTAL_CHECKPOINTS {
            assert!(!platforms_for(seg).is_empty(), "segment {seg} has geometry");
            let s = checkpoint_spawn(seg);
            assert!(
                s.x > prev_spawn,
                "checkpoint {seg} spawn moves forward"
            );
            prev_spawn = s.x;
        }
        let (gx, _) = final_goal_pos();
        assert!(gx > prev_spawn, "final goal is past the last spawn");
    }

    #[test]
    fn checkpoint_flags_sit_inside_their_segments() {
        assert!(checkpoint_flag_pos(1).is_none());
        for seg in 2..=TOTAL_CHECKPOINTS {
            let (fx, _) = checkpoint_flag_pos(seg).expect("flag exists");
            let plats = platforms_for(seg);
            let off = segment_offset(seg);
            let start = off + plats[0].0;
            let end = off + plats[plats.len() - 1].1;
            assert!(fx > start && fx < end, "CP{seg} flag inside segment");
            let s = checkpoint_spawn(seg);
            assert!(s.x > start && s.x < end, "CP{seg} spawn inside segment");
        }
    }

    #[test]
    fn gaps_stay_beatable() {
        // Flat/drop gaps must stay within the ~240px jump range;
        // climbs keep gaps small enough for the ~173px jump height.
        for seg in 1..=TOTAL_CHECKPOINTS {
            let plats = platforms_for(seg);
            for w in plats.windows(2) {
                let (_, prev_max, prev_top) = w[0];
                let (next_min, _, next_top) = w[1];
                let gap = next_min - prev_max;
                let climb = next_top - prev_top;
                assert!(gap <= 215.0, "seg{seg} gap {gap} too wide");
                assert!(gap >= 60.0, "seg{seg} gap {gap} trivial");
                if climb > 50.0 {
                    assert!(
                        gap <= 160.0,
                        "seg{seg} climb {climb} with gap {gap} too hard"
                    );
                }
                assert!(climb <= 100.0, "seg{seg} climb {climb} too high");
            }
        }
        // Transition gaps between segments are short drops/flats.
        for seg in 1..TOTAL_CHECKPOINTS {
            let a = platforms_for(seg);
            let b = platforms_for(seg + 1);
            let (_, a_end, a_top) = a[a.len() - 1];
            let (b_start, _, b_top) = b[0];
            let gap = segment_offset(seg + 1) + b_start - (segment_offset(seg) + a_end);
            assert!(
                (gap - TRANSITION_GAP).abs() < 0.01,
                "transition {seg}->{} is one clean jump",
                seg + 1
            );
            assert!(gap <= 215.0, "transition gap {gap} too wide");
            assert!(
                b_top - a_top <= 50.0,
                "transition must not be a big climb"
            );
        }
    }

    #[test]
    fn difficulty_ramps_monotonically() {
        let mut prev: Option<Difficulty> = None;
        for cp in 1..=TOTAL_CHECKPOINTS {
            let d = difficulty_for(cp);
            if let Some(p) = prev {
                assert!(d.eye_interval <= p.eye_interval, "spawn rate ramps");
                assert!(d.max_eyes >= p.max_eyes, "eye cap ramps");
                assert!(d.eye_speed >= p.eye_speed, "eye speed ramps");
                assert!(d.eye_damage >= p.eye_damage, "eye damage ramps");
            }
            prev = Some(d);
        }
        let easy = difficulty_for(1);
        let hard = difficulty_for(10);
        assert!(easy.eye_interval > hard.eye_interval);
        assert!(easy.max_eyes < hard.max_eyes);
        assert!(easy.eye_speed < hard.eye_speed);
        assert!(easy.eye_damage < hard.eye_damage);
    }
}
