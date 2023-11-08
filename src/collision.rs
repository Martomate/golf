use bevy::{
    gltf::{GltfMesh, GltfNode},
    prelude::*,
};
use bevy_rapier3d::{
    prelude::*,
    rapier::prelude::{Isometry, SharedShape}, na::Translation,
};

pub fn create_collider_from_gltf_node(
    node: &GltfNode,
    gltf_meshes: &Assets<GltfMesh>,
    meshes: &Assets<Mesh>,
    ignore_transform: bool,
) -> Collider {
    let mesh = node.mesh.as_ref().unwrap();
    let gltf_mesh = gltf_meshes.get(mesh).unwrap();
    let handle = &gltf_mesh.primitives[0].mesh;
    let lane_mesh = meshes.get(handle).unwrap();

    let lane_collider =
        Collider::from_bevy_mesh(lane_mesh, &ComputedColliderShape::TriMesh).unwrap();

    let mut tr = if ignore_transform { Transform::IDENTITY } else { node.transform };
    tr.translation /= tr.scale;

    let mut trimesh = lane_collider.as_trimesh().unwrap().raw.clone();
    trimesh.transform_vertices(&Isometry {
        rotation: tr.rotation.into(),
        translation: tr.translation.into(),
    });
    trimesh = trimesh.scaled(&tr.scale.into());
    trimesh.transform_vertices(&Isometry::default());

    Collider::from(SharedShape::new(trimesh))
}
