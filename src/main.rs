use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    pbr::DirectionalLightShadowMap,
    prelude::*,
};
use bevy_prng::ChaCha8Rng;
use bevy_rand::prelude::*;
use bevy_rapier3d::{
    prelude::{Collider, NoUserData, RapierPhysicsPlugin, Restitution, RigidBody, Velocity, ExternalImpulse, Friction, Damping},
    render::RapierDebugRenderPlugin,
};

// These constants are defined in `Transform` units.
// Using the default 2D camera they correspond 1:1 with screen pixels.

const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

fn main() {
    // When building for WASM, print panics to the browser console
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(EntropyPlugin::<ChaCha8Rng>::default())
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 4.0f32,
        })
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(FixedTime::new_from_secs(1.0 / 60.0))
        .add_systems(Startup, (setup_graphics, setup_physics))
        // Add our gameplay simulation systems to the fixed timestep schedule
        .add_systems(Update, (camera_input, jump, bevy::window::close_on_esc))
        .run();
}

#[derive(Component)]
struct Ball;

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
            transform: Transform::from_xyz(0.7, 20.0, 40.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        },
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}

fn setup_physics(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Collider::cuboid(100.0, 0.1, 100.0),
        Friction::new(1.0),
        TransformBundle::from(Transform::from_xyz(0.0, -2.0, 0.0)),
    ));

    commands
        .spawn((
            RigidBody::Dynamic,
            Collider::ball(0.5),
            ExternalImpulse::default(),
            Restitution::coefficient(0.7),
            Friction::new(1.0),
            Damping { linear_damping: 0.95, angular_damping: 0.95 },
            TransformBundle::from(Transform::from_xyz(0.0, 4.0, 0.0)),
        ))
        .insert(Velocity {
            linvel: Vec3::new(1.0, 2.0, 3.0),
            angvel: Vec3::new(0.2, 0.0, 0.0),
        })
        .insert(SceneBundle {
            scene: asset_server.load("models/sphere.gltf#Scene0"),
            ..default()
        })
        .insert(Ball);
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
    mut query: Query<(&mut CameraController, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut controller, mut transform) in query.iter_mut() {
        for wheel in mouse_wheel.iter() {
            controller.zoom -= wheel.y * 0.01;
        }
        if buttons.pressed(MouseButton::Left) {
            for mouse in mouse_motion.iter() {
                let delta = mouse.delta * time.delta_seconds() * 0.3;
                controller.rotation *= Quat::from_euler(EulerRot::XYZ, -delta.y, -delta.x, 0.0);
            }
        }
        transform.translation = controller.rotation * Vec3::Z * controller.zoom;
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

fn jump(keyboard_input: Res<Input<KeyCode>>, mut q_ball: Query<(&mut ExternalImpulse, &Velocity)>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for (mut ball_impulse, ball_velocity) in &mut q_ball {
            if ball_velocity.linvel.y.abs() <= 0.05 {
                ball_impulse.impulse.y += 5.0;
            }
        }
    }
}
