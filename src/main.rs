use std::f32::consts::PI;

use bevy::{
    gltf::{GltfMesh, GltfNode},
    input::mouse::{MouseMotion, MouseWheel},
    pbr::DirectionalLightShadowMap,
    prelude::*,
    scene::SceneInstance,
};
use bevy_rapier3d::{prelude::*, render::RapierDebugRenderPlugin};
use rand::Rng;

mod collision;

// These constants are defined in `Transform` units.
// Using the default 2D camera they correspond 1:1 with screen pixels.

const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    InGame,
}

fn main() {
    // When building for WASM, print panics to the browser console
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_state::<AppState>()
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 4.0f32,
        })
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(FixedTime::new_from_secs(1.0 / 60.0))
        .insert_resource(AssetsLoading::default())
        .insert_resource(GameState::new(4))
        .add_systems(Startup, setup_graphics)
        .add_systems(OnEnter(AppState::Loading), load_assets)
        .add_systems(OnEnter(AppState::InGame), load_level)
        // Add our gameplay simulation systems to the fixed timestep schedule
        .add_systems(
            Update,
            (
                check_assets_ready,
                camera_input,
                move_camera_to_ball,
                keyboard_input,
                update_shoot_power_indicator,
                check_ball_in_hole,
                customize_scene_materials,
                stop_ball_from_spinning_forever,
            ),
        );

    if cfg!(debug_assertions) {
        app.add_plugins(RapierDebugRenderPlugin::default());
    }

    app.run();
}

#[derive(Component)]
struct Ball {
    player_id: u32,
    hits: u32,
}

#[derive(Debug, Clone, PartialEq)]
enum BallSpin {
    Left,
    Right,
}

#[derive(Component, Debug, Clone, PartialEq, Default)]
struct ShootSettings {
    power: f32,
    angle: f32,
    spin: Option<BallSpin>,
}

#[derive(Component)]
struct ShootPowerIndicator;

#[derive(Component)]
struct Hole;

#[derive(Resource)]
struct GameState {
    num_players: u32,
    current_player: u32,
    players: Vec<PlayerData>,
}

impl GameState {
    fn new(num_players: u32) -> Self {
        GameState {
            num_players,
            current_player: 0,
            players: (0..num_players).map(|_| PlayerData::default()).collect(),
        }
    }
}

#[derive(Default)]
struct PlayerData {
    scores: Vec<u32>,
}

#[derive(Component)]
struct NeedsColorChange(Color);

#[derive(Resource, Default)]
struct AssetsLoading(Vec<HandleUntyped>);

fn load_assets(server: Res<AssetServer>, mut loading: ResMut<AssetsLoading>) {
    let scene: Handle<Scene> = server.load("models/lane.gltf#Scene0");
    loading.0.push(scene.clone_untyped());
    let scene: Handle<Scene> = server.load("models/sphere.gltf#Scene0");
    loading.0.push(scene.clone_untyped());
    let scene: Handle<Scene> = server.load("models/cube.gltf#Scene0");
    loading.0.push(scene.clone_untyped());
    let scene: Handle<Scene> = server.load("models/torus.gltf#Scene0");
    loading.0.push(scene.clone_untyped());
    let scene: Handle<Scene> = server.load("models/cone.gltf#Scene0");
    loading.0.push(scene.clone_untyped());
}

fn check_assets_ready(
    server: Res<AssetServer>,
    loading: Res<AssetsLoading>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    use bevy::asset::LoadState;

    if server.get_group_load_state(loading.0.iter().map(|a| a.id())) == LoadState::Loaded {
        next_state.set(AppState::InGame);
    }
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        CameraController {
            rotation: Quat::from_rotation_y(-PI * 0.5),
            zoom: 0.0,
        },
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 1.5, 1.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        },
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 20000.0,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 1.5, -1.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });
}

fn load_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    nodes: Res<Assets<GltfNode>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    game_state: Res<GameState>,
) {
    commands.spawn((
        Collider::cuboid(100.0, 0.1, 100.0),
        Friction::new(1.0),
        TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.0)),
    ));

    let mut lane_colliders = Vec::new();
    for i in 0..5 {
        let node_handle: Handle<GltfNode> = asset_server.load(format!("models/lane.gltf#Node{i}"));
        let node = nodes.get(&node_handle).unwrap();

        lane_colliders.push(collision::create_collider_from_gltf_node(
            node,
            &gltf_meshes,
            &meshes,
        ));
    }

    commands
        .spawn((
            RigidBody::Fixed,
            SceneBundle {
                scene: asset_server.load("models/lane.gltf#Scene0"),
                transform: Transform::from_xyz(0.0, 0.3, 0.0)
                    .with_rotation(Quat::from_rotation_y(-PI / 2.0))
                    .with_scale(Vec3::new(0.5, 0.5, 0.5)),
                ..default()
            },
            Friction::new(1.0),
        ))
        .with_children(|parent| {
            for lane_collider in lane_colliders {
                parent.spawn((lane_collider, TransformBundle::IDENTITY));
            }
        });

    commands.spawn((
        Collider::cylinder(0.02, 0.05),
        TransformBundle::from_transform(Transform::from_xyz(10.0, 0.3 - 0.025 + 0.03, 0.0)),
        Sensor,
        Hole,
    ));

    let mut rng = rand::thread_rng();
    for player_id in 0..game_state.num_players {
        let shape = match rng.gen_range(0..=2) {
            0 => BallShape::Sphere,
            1 => BallShape::Cube,
            _ => BallShape::Cone,
        };
        spawn_ball(
            &mut commands,
            &asset_server,
            player_id,
            rng.gen_range(-0.4..0.4),
            rng.gen_range(-0.4..0.0),
            Color::hsl(rng.gen_range(0.0..360.0), 1.0, 0.5),
            shape,
        );
    }

    commands.spawn((
        ShootPowerIndicator,
        PbrBundle {
            mesh: meshes.add(shape::Cube::new(1.0).into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.0)
                .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.0))
                .with_scale(Vec3::new(0.0, 0.0, 0.0)),
            material: materials.add(StandardMaterial {
                base_color: Color::CYAN,
                ..Default::default()
            }),
            ..Default::default()
        },
    ));
}

fn customize_scene_materials(
    mut commands: Commands,
    unloaded_instances: Query<(Entity, &SceneInstance, &NeedsColorChange)>,
    mut handles: Query<(Entity, &mut Handle<StandardMaterial>)>,
    mut pbr_materials: ResMut<Assets<StandardMaterial>>,
    scene_manager: Res<SceneSpawner>,
) {
    for (entity, instance, requesed_change) in unloaded_instances.iter() {
        if scene_manager.instance_is_ready(**instance) {
            commands.entity(entity).remove::<NeedsColorChange>();
        }
        // Iterate over all entities in scene (once it's loaded)
        let mut handles = handles.iter_many_mut(scene_manager.iter_instance_entities(**instance));
        while let Some((_, mut material_handle)) = handles.fetch_next() {
            let Some(material) = pbr_materials.get(&material_handle) else {
                continue;
            };
            let mut new_material = material.clone();
            new_material.base_color = requesed_change.0;

            *material_handle = pbr_materials.add(new_material);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BallShape {
    Sphere,
    Cube,
    Cone,
}

fn spawn_ball(
    commands: &mut Commands,
    asset_server: &AssetServer,
    player_id: u32,
    offset_sideways: f32,
    offset_along: f32,
    color: Color,
    shape: BallShape,
) {
    let model_file = match shape {
        BallShape::Sphere => "sphere",
        BallShape::Cube => "cube",
        BallShape::Cone => "cone",
    };
    let scene_handle = asset_server.load(format!("models/{}.gltf#Scene0", model_file));
    //let node_handle = asset_server.load("models/sphere.gltf#Node0");

    let collider = match shape {
        BallShape::Sphere => Collider::ball(0.025),
        BallShape::Cube => Collider::cuboid(0.025, 0.025, 0.025),
        BallShape::Cone => Collider::cone(0.025, 0.025),
    };

    let r = 0.025;
    let density = 1.0;
    let mass = r * r * r * 8.0 * density;

    let principal_inertia = Vec3::new(1.0, 1.0, 1.0) * 3.0 / 10.0 * r * r * mass;

    commands
        .spawn((
            RigidBody::Dynamic,
            collider,
            ExternalImpulse::default(),
            ExternalForce::default(),
            Restitution {
                coefficient: 0.5,
                combine_rule: CoefficientCombineRule::Max,
            },
            Friction {
                coefficient: 1.0,
                combine_rule: CoefficientCombineRule::Max,
            },
            ColliderMassProperties::MassProperties(MassProperties {
                local_center_of_mass: Vec3::ZERO,
                mass,
                principal_inertia_local_frame: Quat::IDENTITY,
                principal_inertia,
            }),
            ReadMassProperties::default(),
            Damping {
                linear_damping: 0.4,
                angular_damping: 0.9,
            },
            Ccd::enabled(),
        ))
        .insert(Velocity {
            linvel: Vec3::new(0.0, 0.0, 0.0),
            angvel: Vec3::new(0.0, 0.0, 0.0),
        })
        .insert(SceneBundle {
            scene: scene_handle,
            transform: Transform::from_xyz(offset_along, 1.0, offset_sideways)
                * Transform::from_rotation(Quat::from_rotation_x(PI / 3.0)),
            ..default()
        })
        .insert(NeedsColorChange(color))
        .insert(Ball { player_id, hits: 0 })
        .insert(ShootSettings::default());
}

fn stop_ball_from_spinning_forever(
    mut q_ball: Query<(&mut ExternalImpulse, &Velocity, &ReadMassProperties), With<Ball>>,
) {
    for (mut f, vel, mass) in q_ball.iter_mut() {
        if vel.linvel.length() < 0.025 {
            f.impulse -= vel.linvel * mass.0.mass * 0.9;
            f.torque_impulse = -vel.angvel * mass.0.principal_inertia * 0.9;
        }
    }
}

fn check_ball_in_hole(
    mut commands: Commands,
    rapier_context: Res<RapierContext>,
    q_hole: Query<Entity, With<Hole>>,
    q_ball: Query<(Entity, &Velocity, &Ball), Without<Hole>>,
    mut game_state: ResMut<GameState>,
) {
    for hole_entity in q_hole.iter() {
        for (ball_entity, ball_velocity, ball) in q_ball.iter() {
            println!(
                "{:?}: {:?} - {:?}",
                ball_entity,
                ball_velocity.linvel.length(),
                ball_velocity.angvel.length()
            );
            if ball_velocity.linvel.length() < 0.01
                && rapier_context.intersection_pair(hole_entity, ball_entity) == Some(true)
            {
                game_state.players[ball.player_id as usize]
                    .scores
                    .push(ball.hits);
                println!("Player {} finished in {} moves", ball.player_id, ball.hits);

                commands.entity(ball_entity).despawn_recursive();

                game_state.current_player =
                    (game_state.current_player + 1) % game_state.num_players;

                if game_state.players.iter().all(|p| p.scores.len() == 1) {
                    println!("Level 1 completed!");
                }
            }
        }
    }
}

fn update_shoot_power_indicator(
    mut q_indicator: Query<&mut Transform, (With<ShootPowerIndicator>, Without<Ball>)>,
    q_ball: Query<(&Transform, &ShootSettings, &Ball)>,
    game_state: Res<GameState>,
) {
    if let Some((ball_transform, shoot_settings, _)) = q_ball
        .iter()
        .find(|(_, _, ball)| ball.player_id == game_state.current_player)
    {
        let length = shoot_settings.power * 0.1;
        let pos = ball_transform.translation;
        let angle = shoot_settings.angle;
        let scale = Vec3::new(length, if length == 0.0 { 0.0 } else { 0.005 }, 0.02);

        let t1 = Transform::from_xyz(length * 0.5, 0.0, 0.0).with_scale(scale);
        let t2 = Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(angle));

        if let Ok(mut indicator_transform) = q_indicator.get_single_mut() {
            *indicator_transform = t2 * t1;
        }
    }
}

#[derive(Component)]
struct CameraController {
    rotation: Quat,
    zoom: f32,
}

fn camera_input(
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    buttons: Res<Input<MouseButton>>,
    mut query: Query<&mut CameraController>,
    time: Res<Time>,
) {
    for mut controller in query.iter_mut() {
        for wheel in mouse_wheel.iter() {
            controller.zoom += wheel.y * 0.001;
        }
        if buttons.pressed(MouseButton::Left) {
            for mouse in mouse_motion.iter() {
                let delta = mouse.delta * time.delta_seconds() * 0.3;
                controller.rotation *= Quat::from_euler(EulerRot::XYZ, -delta.y, -delta.x, 0.0);
            }
        }
    }
}

fn move_camera_to_ball(
    mut query: Query<(&CameraController, &mut Transform), Without<Ball>>,
    q_ball: Query<(&Transform, &Ball)>,
    game_state: Res<GameState>,
) {
    if let Ok((controller, mut transform)) = query.get_single_mut() {
        if let Some((ball_transform, _)) = q_ball
            .iter()
            .find(|(_, ball)| ball.player_id == game_state.current_player)
        {
            let ball_pos = ball_transform.translation;
            let mut look = controller.rotation * Vec3::Z;
            look.y = 0.3;
            look = look.normalize();
            transform.translation = ball_pos + look * (-controller.zoom).exp();
            transform.look_at(ball_pos, Vec3::Y);
        }
    }
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q_ball: Query<(
        &mut ExternalImpulse,
        &ReadMassProperties,
        &Velocity,
        &mut ShootSettings,
        &mut Ball,
    )>,
    game_state: Res<GameState>,
) {
    if let Some((mut ball_impulse, &ball_mass, &ball_velocity, mut shoot, mut ball)) = q_ball
        .iter_mut()
        .find(|(_, _, _, _, ball)| ball.player_id == game_state.current_player)
    {
        if ball_velocity.linvel.length() < 0.01 {
            let max_power = 10.0;
            let power_speed = 0.1;
            let angle_speed = 0.5 / 180.0 * PI;

            if keys.pressed(KeyCode::W) {
                shoot.power += power_speed;
            }
            if keys.pressed(KeyCode::S) {
                shoot.power -= power_speed;
            }
            if keys.pressed(KeyCode::A) {
                shoot.angle += angle_speed;
            }
            if keys.pressed(KeyCode::D) {
                shoot.angle -= angle_speed;
            }
            if keys.just_pressed(KeyCode::Q) {
                if let Some(BallSpin::Left) = shoot.spin {
                    shoot.spin = None;
                } else {
                    shoot.spin = Some(BallSpin::Left);
                }
            }
            if keys.just_pressed(KeyCode::E) {
                if let Some(BallSpin::Right) = shoot.spin {
                    shoot.spin = None;
                } else {
                    shoot.spin = Some(BallSpin::Right);
                }
            }
            if keys.just_pressed(KeyCode::Escape) {
                *shoot = ShootSettings::default();
            }

            shoot.power = shoot.power.max(0.0).min(max_power);

            shoot.angle %= 2.0 * PI;
            if shoot.angle < 0.0 {
                shoot.angle += 2.0 * PI;
            }
        }

        if keys.just_pressed(KeyCode::Space) {
            if ball_velocity.linvel.length() < 0.01 && *shoot != ShootSettings::default() {
                let rot = Quat::from_euler(EulerRot::XYZ, 0.0, shoot.angle, 0.0);
                let transform = Transform::from_rotation(rot);
                let dir = transform * Vec3::X;

                let power_multiplier = 1.0 * ball_mass.0.mass;
                let shot = dir * shoot.power * power_multiplier;
                ball_impulse.impulse.x += shot.x;
                ball_impulse.impulse.y += shot.y;
                ball_impulse.impulse.z += shot.z;

                let torqe_magnitude = 1.0 * ball_mass.0.mass;
                let torque_amount = match shoot.spin {
                    Some(BallSpin::Left) => -torqe_magnitude,
                    Some(BallSpin::Right) => torqe_magnitude,
                    None => 0.0,
                };
                ball_impulse.torque_impulse.y += torque_amount;
                ball_impulse.torque_impulse.x += torque_amount;

                ball.hits += 1;

                *shoot = ShootSettings::default();
            } else if ball_velocity.linvel.y.abs() <= 0.05 {
                ball_impulse.impulse.y += 7.0 * ball_mass.0.mass;
            }
        }
    }
}
