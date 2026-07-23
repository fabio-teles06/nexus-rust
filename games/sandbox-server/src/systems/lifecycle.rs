use bevy_ecs::prelude::*;
use engine_core::ClientId;
use engine_physics::{PhysicsBody, PhysicsWorld};
use engine_server::Outgoing;
use sandbox_shared::ServerMessage;

use crate::{
    components::{NetworkEntity, PlayerOwner},
    resources::{ConnectedClients, Outbox, PlayerRegistry},
};

pub(crate) fn process_connected(client_id: ClientId, world: &mut World) {
    world
        .resource_mut::<ConnectedClients>()
        .0
        .insert(client_id);
}

pub(crate) fn process_disconnected(client_id: ClientId, world: &mut World) {
    world
        .resource_mut::<ConnectedClients>()
        .0
        .remove(&client_id);

    let network_id = world
        .resource_mut::<PlayerRegistry>()
        .0
        .remove(&client_id);

    let Some(network_id) = network_id else {
        return;
    };

    let target = {
        let mut query = world.query::<(
            Entity,
            &PlayerOwner,
            &NetworkEntity,
            &PhysicsBody,
        )>();

        query.iter(world).find_map(|(entity, owner, network, body)| {
            (owner.0 == client_id && network.0 == network_id)
                .then_some((entity, *body))
        })
    };

    if let Some((entity, physics_body)) = target {
        world
            .resource_mut::<PhysicsWorld>()
            .remove_body(physics_body);
        let _ = world.despawn(entity);
    }

    world
        .resource_mut::<Outbox>()
        .0
        .push(Outgoing::Broadcast(ServerMessage::DespawnBatch(vec![
            network_id,
        ])));
}
