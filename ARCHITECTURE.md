# Responsabilidades

## `engine-physics`
Backend Rapier 3D autoritativo. Mantém `PhysicsPipeline`, `RigidBodySet`,
`ColliderSet`, broad phase, narrow phase, joints e CCD. O servidor executa um
único passo por tick e sincroniza os `Transform` ECS depois da simulação.

As entidades guardam apenas:

```text
PhysicsBody
├── RigidBodyHandle
└── ColliderHandle
```

O piso e as paredes são colliders estáticos. Jogadores usam rigid-bodies
dinâmicos com rotações bloqueadas e CCD habilitado.

## `engine-assets`
Handles tipados e armazenamento de assets. O cliente associa `MeshHandle` e `MaterialHandle` às entidades ECS.

## `engine-network`
Hub local multi-cliente. Conexões geram `Connected`, mensagens levam `ClientId`, e `Drop` gera `Disconnected`.

## Replicação
O servidor envia `SpawnBatch`, `SnapshotBatch` e `DespawnBatch`. A frequência de snapshots é 15 Hz, independente dos 30 TPS.

## Prediction e reconciliação
O cliente aplica imediatamente cada input ao `SimulationTransform`, guarda-o em `PredictionState.pending` e, ao receber um snapshot com `last_processed_input`, remove inputs confirmados e reaplica os restantes sobre o estado autoritativo do Rapier.

A prediction atual aproxima apenas o movimento horizontal. Gravidade e colisões permanecem autoritativas no servidor e são corrigidas pelos snapshots.

## Simulação x renderização
- `NetworkTransform`: duas amostras autoritativas para interpolação.
- `SimulationTransform`: estado previsto ou autoritativo usado pelo cliente.
- `RenderTransform`: estado suave consumido pelo renderizador.
