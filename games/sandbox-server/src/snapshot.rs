use engine_ecs::prelude::*;
use sandbox_shared::TransformSnapshot;

pub(crate) fn snapshot_from_transform(
    transform: &Transform,
) -> TransformSnapshot {
    TransformSnapshot {
        translation: transform.translation.to_array(),
        rotation: transform.rotation.to_array(),
        scale: transform.scale.to_array(),
    }
}