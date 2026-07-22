use engine_ecs::prelude::*;
use sandbox_shared::{EntityKind, NetworkId, TransformSnapshot};

use crate::{
    client::SandboxClient,
    components::{ClientEntityKind, ClientNetworkId, LocalPlayer, ReplicatedEntity},
};

impl SandboxClient {
    pub(crate) fn spawn_or_update_entity(
        &mut self,
        network_id: NetworkId,
        kind: EntityKind,
        snapshot: TransformSnapshot,
    ) {
        let transform = transform_from_snapshot(snapshot);

        if let Some(&entity) = self.entities.get(&network_id) {
            self.world
                .entity_mut(entity)
                .insert((transform, ClientEntityKind(kind)));

            return;
        }

        let entity = self
            .world
            .spawn((
                ReplicatedEntity,
                ClientNetworkId(network_id),
                ClientEntityKind(kind),
                transform,
            ))
            .id();

        self.entities.insert(network_id, entity);

        if self.local_player == Some(network_id) {
            self.world.entity_mut(entity).insert(LocalPlayer);
        }
    }

    pub(crate) fn update_entity_transform(
        &mut self,
        network_id: NetworkId,
        snapshot: TransformSnapshot,
    ) {
        let Some(&entity) = self.entities.get(&network_id) else {
            return;
        };
        
        self.world
            .entity_mut(entity)
            .insert(transform_from_snapshot(snapshot));
    }

    pub(crate) fn despawn_entity(&mut self, network_id: NetworkId) {
        let Some(entity) = self.entities.remove(&network_id) else {
            return;
        };

        let _ = self.world.despawn(entity);

        if self.local_player == Some(network_id) {
            self.local_player = None;
        }
    }

    pub(crate) fn mark_local_player_if_spawned(&mut self) {
        let Some(network_id) = self.local_player else {
            return;
        };

        let Some(&entity) = self.entities.get(&network_id) else {
            return;
        };

        self.world.entity_mut(entity).insert(LocalPlayer);
    }
}

fn transform_from_snapshot(snapshot: TransformSnapshot) -> Transform {
    Transform {
        translation: Vec3::from_array(snapshot.translation),
        rotation: Quat::from_array(snapshot.rotation),
        scale: Vec3::from_array(snapshot.scale),
    }
}
