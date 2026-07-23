use engine_ecs::Transform;
use sandbox_shared::TransformSnapshot;

pub(crate) fn snapshot(transform: &Transform) -> TransformSnapshot {
    TransformSnapshot {
        translation: transform.translation.to_array(),
        rotation: transform.rotation.to_array(),
        scale: transform.scale.to_array(),
    }
}
