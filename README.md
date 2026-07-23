# Ferrum Engine — arquitetura cliente/servidor em Rust

Projeto-base de uma engine **code-first**, sem editor, com servidor autoritativo interno no estilo Minecraft.

## Funcionalidades incluídas

- `bevy_ecs` para ECS no servidor e no cliente.
- `glam` para matemática da engine.
- `wgpu 29.0.3` + `winit 0.30` para renderização e janela.
- **Rapier 3D 0.34.0** como backend físico autoritativo do servidor.
- Corpos dinâmicos para jogadores, gravidade, CCD, piso e paredes com colliders estáticos.
- Assets tipados com `Handle<T>` e armazenamento generacional simples.
- Transporte local com vários clientes, eventos de conexão/desconexão e broadcast.
- Replicação em lote (`SnapshotBatch`) em frequência independente do tick do servidor.
- Client prediction com histórico de inputs e reconciliação por `last_processed_input`.
- Separação entre `NetworkTransform`, `SimulationTransform` e `RenderTransform`.
- Interpolação para entidades remotas.
- Exemplo gráfico e demonstração headless com dois clientes.

## Executar

```bash
cargo run -p sandbox-client
```

Controles: `WASD` ou setas; `Esc` fecha.

Demonstração com dois clientes locais:

```bash
cargo run -p sandbox-multi-client
```

## Arquitetura

```text
Input local
  -> PlayerInput(sequence)
  -> servidor autoritativo
  -> velocidade horizontal no RigidBody do Rapier
  -> PhysicsPipeline::step
  -> Transform ECS sincronizado do corpo físico
  -> SnapshotBatch(last_processed_input)
  -> reconciliação do jogador local
  -> interpolação dos jogadores remotos
  -> RenderTransform
  -> wgpu
```

## Física

A crate `engine-physics` encapsula o Rapier. Entidades ECS armazenam apenas um
`PhysicsBody`, com os handles do rigid-body e collider. O mundo físico, pipeline,
colliders, joints e estruturas de detecção ficam centralizados no recurso
`PhysicsWorld` do servidor.

Isso mantém o Rapier fora do protocolo de rede e do renderizador, além de impedir
que a simulação seja executada mais de uma vez por tick quando existem vários
jogadores.
