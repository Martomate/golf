use std::f32::consts::PI;

use bevy::{
    gltf::{GltfMesh, GltfNode},
    input::mouse::{MouseMotion, MouseWheel},
    pbr::DirectionalLightShadowMap,
    prelude::*,
};
use bevy_prng::ChaCha8Rng;
use bevy_rand::prelude::*;
use bevy_rapier3d::{prelude::*, render::RapierDebugRenderPlugin, rapier::prelude::{Isometry, SharedShape}};

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
        .add_plugins(EntropyPlugin::<ChaCha8Rng>::default())
        .add_state::<AppState>()
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 4.0f32,
        })
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(FixedTime::new_from_secs(1.0 / 60.0))
        .insert_resource(AssetsLoading(Vec::new()))
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
            ),
        );

    if cfg!(debug_assertions) {
        app.add_plugins(RapierDebugRenderPlugin::default());
    }

    app.run();
}

#[derive(Component)]
struct Ball;

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

#[derive(Resource)]
struct AssetsLoading(Vec<HandleUntyped>);

fn load_assets(server: Res<AssetServer>, mut loading: ResMut<AssetsLoading>) {
    let scene: Handle<Scene> = server.load("models/lane.gltf#Scene0");
    loading.0.push(scene.clone_untyped());
}

// TODO: load level when the models have finished loading (listen to AssetEvents)

fn check_assets_ready(
    server: Res<AssetServer>,
    loading: Res<AssetsLoading>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    use bevy::asset::LoadState;

    match server.get_group_load_state(loading.0.iter().map(|a| a.id())) {
        LoadState::Failed => {
            // one of our assets had an error
        }
        LoadState::Loaded => next_state.set(AppState::InGame),
        _ => {
            // NotLoaded/Loading: not fully ready yet
        }
    }
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        CameraController {
            rotation: Quat::IDENTITY,
            zoom: 20.0,
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
    meshes: Res<Assets<Mesh>>,
) {
    commands.spawn((
        Collider::cuboid(100.0, 0.1, 100.0),
        Friction::new(1.0),
        TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.0)),
    ));

    let node_handle: Handle<GltfNode> = asset_server.load("models/lane.gltf#Node1");
    let node = nodes.get(&node_handle).unwrap();

    let mesh = node.mesh.as_ref().unwrap();
    let gltf_mesh = gltf_meshes.get(mesh).unwrap();
    let handle = &gltf_mesh.primitives[0].mesh;
    let lane_mesh = meshes.get(handle).unwrap();

    let lane_collider =
        Collider::from_bevy_mesh(lane_mesh, &ComputedColliderShape::TriMesh).unwrap();

    /*  Collider::compound(vec![
        (
            Vec3::new(0.0, 0.0, -10.0),
            Quat::IDENTITY,
            Collider::cuboid(1.0, 0.05, 11.0),
        ),
        (
            Vec3::new(1.05, 0.05, -10.0),
            Quat::IDENTITY,
            Collider::cuboid(0.05, 0.1, 11.0),
        ),
        (
            Vec3::new(-1.05, 0.05, -10.0),
            Quat::IDENTITY,
            Collider::cuboid(0.05, 0.1, 11.0),
        ),
        (
            Vec3::new(0.0, 0.05, 1.05),
            Quat::IDENTITY,
            Collider::cuboid(1.1, 0.1, 0.05),
        ),
        (
            Vec3::new(0.0, 0.05, -21.05),
            Quat::IDENTITY,
            Collider::cuboid(1.1, 0.1, 0.05),
        ),
    ]*/

    let mut tr = node.transform;
    tr.translation /= tr.scale;

    let mut trimesh = lane_collider.as_trimesh().unwrap().raw.clone();
    trimesh.transform_vertices(&Isometry {rotation: tr.rotation.into(), translation: tr.translation.into()});
    trimesh = trimesh.scaled(&tr.scale.into());
    trimesh.transform_vertices(&Isometry::default());

    let lane_collider = Collider::from(SharedShape::new(trimesh));

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
        ))
        .with_children(|parent| {
            parent.spawn((
                lane_collider,
                TransformBundle::IDENTITY,
            ));
        });

    commands
        .spawn((
            RigidBody::Dynamic,
            Collider::ball(0.025),
            ExternalImpulse::default(),
            Restitution::coefficient(0.7),
            Friction::new(1.0),
            Damping {
                linear_damping: 0.95,
                angular_damping: 0.95,
            },
            Ccd::enabled(),
        ))
        .insert(Velocity {
            linvel: Vec3::new(0.0, 0.0, 0.0),
            angvel: Vec3::new(0.0, 0.0, 0.0),
        })
        .insert(SceneBundle {
            scene: asset_server.load("models/sphere.gltf#Scene0"),
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        })
        .insert(Ball)
        .insert(ShootSettings::default());
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
            controller.zoom -= wheel.y * 0.01;
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
    q_ball: Query<&Transform, With<Ball>>,
) {
    if let Ok((controller, mut transform)) = query.get_single_mut() {
        if let Ok(ball) = q_ball.get_single() {
            let ball_pos = ball.translation;
            let mut look = controller.rotation * Vec3::Z;
            look.y = 0.5;
            look = look.normalize();
            transform.translation = ball_pos + look * controller.zoom;
            transform.look_at(ball_pos, Vec3::Y);
        }
    }
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q_ball: Query<(&mut ExternalImpulse, &Velocity)>,
    mut shoot_settings: Query<&mut ShootSettings>,
) {
    if let Ok(mut shoot) = shoot_settings.get_single_mut() {
        let shoot_before = shoot.clone();

        let max_power = 10.0;
        let power_speed = 0.1;
        let angle_speed = 2.0 / 180.0 * PI;

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

        if *shoot != shoot_before {
            println!("{:?}", shoot);
        }

        if keys.just_pressed(KeyCode::Space) {
            if *shoot != ShootSettings::default() {
                for (mut ball_impulse, _) in &mut q_ball {
                    let rot = Quat::from_euler(EulerRot::XYZ, 0.0, shoot.angle, 0.0);
                    let transform = Transform::from_rotation(rot);
                    let dir = transform * Vec3::X;

                    let power_multiplier = 1.0e-4;
                    let shot = dir * shoot.power * power_multiplier;
                    ball_impulse.impulse.x += shot.x;
                    ball_impulse.impulse.y += shot.y;
                    ball_impulse.impulse.z += shot.z;

                    let torqe_magnitude = 1.0e-3;
                    let torque_amount = match shoot.spin {
                        Some(BallSpin::Left) => -torqe_magnitude,
                        Some(BallSpin::Right) => torqe_magnitude,
                        None => 0.0,
                    };
                    ball_impulse.torque_impulse.y += torque_amount;
                    ball_impulse.torque_impulse.x += torque_amount;
                }

                *shoot = ShootSettings::default();
            } else {
                for (mut ball_impulse, ball_velocity) in &mut q_ball {
                    if ball_velocity.linvel.y.abs() <= 0.05 {
                        ball_impulse.impulse.y += 5.0e-4;
                    }
                }
            }
        }
    }
}
