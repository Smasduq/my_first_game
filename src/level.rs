use bevy::prelude::*;

/// Solid ground. `size` is full width/height, centered on the Transform.
#[derive(Component)]
pub struct Platform {
    pub size: Vec2,
}

/// Finish flag.
#[derive(Component)]
pub struct Goal;

/// Where the player (re)spawns.
#[derive(Resource)]
pub struct SpawnPoint(pub Vec3);

/// Deaths + win state.
#[derive(Resource, Default)]
pub struct LevelState {
    pub deaths: u32,
    pub won: bool,
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

/// Platforms as (x_min, x_max, top_y).
const PLATFORMS: [(f32, f32, f32); 4] = [
    (-350.0, 350.0, 0.0),
    (490.0, 890.0, 0.0),
    (1050.0, 1400.0, 70.0),
    (1580.0, 2280.0, 0.0),
];

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

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpawnPoint(Vec3::new(-250.0, 150.0, 0.0)))
            .insert_resource(LevelState::default())
            .add_systems(
                Startup,
                (
                    spawn_platforms,
                    spawn_goal,
                    spawn_background,
                    spawn_trees,
                    spawn_lanterns,
                    spawn_fog,
                ),
            )
            .add_systems(Update, (scroll_background, drift_fog, flicker_lanterns));
    }
}

fn spawn_platforms(mut commands: Commands) {
    // Mountain Dusk palette: near-black purple rock, sunlit red-orange lip.
    let ground = Color::srgb(0.10, 0.06, 0.15);
    let under_lip = Color::srgb(0.05, 0.03, 0.08);
    let edge = Color::srgb(0.52, 0.21, 0.19);

    for (x_min, x_max, top) in PLATFORMS {
        let w = x_max - x_min;
        let cx = (x_min + x_max) * 0.5;
        let h = 90.0;
        commands.spawn((
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
            Sprite {
                color: edge,
                custom_size: Some(Vec2::new(w, 8.0)),
                ..default()
            },
            Transform::from_xyz(cx, top - 4.0, 0.5),
        ));
        commands.spawn((
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
                Sprite {
                    color: Color::srgb(0.13, 0.08, 0.17),
                    custom_size: Some(Vec2::new(3.0, seam_height)),
                    ..default()
                },
                Transform::from_xyz(seam_x, top - 30.0, 0.6),
            ));
            commands.spawn((
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

fn spawn_goal(mut commands: Commands) {
    let goal_x = 2200.0;
    let top = 0.0;
    // Pole.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.6, 0.58, 0.55),
            custom_size: Some(Vec2::new(8.0, 170.0)),
            ..default()
        },
        Transform::from_xyz(goal_x, top + 85.0, 1.0),
    ));
    // Banner.
    commands.spawn((
        Goal,
        Sprite {
            color: Color::srgb(0.55, 0.12, 0.14),
            custom_size: Some(Vec2::new(70.0, 44.0)),
            ..default()
        },
        Transform::from_xyz(goal_x + 39.0, top + 148.0, 1.0),
    ));
    commands.spawn((
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

/// Black dead-tree silhouettes on the platforms (foreground depth over the art).
fn spawn_trees(mut commands: Commands) {
    let black = Color::srgb(0.008, 0.008, 0.015);

    let tree_x = [-200.0, 150.0, 620.0, 800.0, 1180.0, 1700.0, 2050.0];
    for (i, x) in tree_x.iter().enumerate() {
        let top = if (1000.0..1450.0).contains(x) {
            70.0
        } else {
            0.0
        };
        let h_var = 110.0 + (i as f32 % 3.0) * 25.0;
        // Trunk.
        commands.spawn((
            Sprite {
                color: black,
                custom_size: Some(Vec2::new(12.0, h_var)),
                ..default()
            },
            Transform::from_xyz(*x, top + h_var * 0.5, -5.0),
        ));
        // Two branches.
        for (side, ang) in [(-1.0, 0.6), (1.0, -0.55)] {
            commands.spawn((
                Sprite {
                    color: black,
                    custom_size: Some(Vec2::new(8.0, 55.0)),
                    ..default()
                },
                Transform {
                    translation: Vec3::new(*x + side * 16.0, top + h_var - 20.0, -5.0),
                    rotation: Quat::from_rotation_z(ang * side),
                    ..default()
                },
            ));
        }
    }
}

fn spawn_fog(mut commands: Commands) {
    let fog = Color::srgba(0.45, 0.45, 0.55, 0.07);
    let xs = [
        -500.0, -200.0, 100.0, 420.0, 700.0, 1000.0, 1300.0, 1600.0, 1900.0, 2200.0,
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

fn spawn_lanterns(mut commands: Commands) {
    for (i, (x, top)) in [(190.0, 0.0), (720.0, 0.0), (1260.0, 70.0), (1860.0, 0.0)]
        .into_iter()
        .enumerate()
    {
        let metal = Color::srgb(0.11, 0.08, 0.12);
        commands.spawn((
            Sprite {
                color: metal,
                custom_size: Some(Vec2::new(5.0, 30.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 15.0, 2.0),
        ));
        commands.spawn((
            Sprite {
                color: metal,
                custom_size: Some(Vec2::new(18.0, 4.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 29.0, 2.0),
        ));
        commands.spawn((
            LanternGlow {
                phase: i as f32 * 1.7,
            },
            Sprite {
                color: Color::srgba(0.92, 0.49, 0.15, 0.14),
                custom_size: Some(Vec2::new(30.0, 34.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 38.0, 2.1),
        ));
        commands.spawn((
            Sprite {
                color: Color::srgb(0.96, 0.61, 0.24),
                custom_size: Some(Vec2::new(9.0, 14.0)),
                ..default()
            },
            Transform::from_xyz(x, top + 38.0, 2.2),
        ));
        commands.spawn((
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
