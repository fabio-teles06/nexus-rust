# Arquitetura

O protótipo mantém o renderer desacoplado do mundo e das regras do jogo.

```text
winit events
    ↓
App ───────────────→ DebugGui
 │
 ├──→ Game
 │    ├── Camera
 │    ├── VoxelWorld
 │    ├── PhysicsWorld (Rapier)
 │    └── EcsScene (bevy_ecs)
 │
 └──→ Renderer (wgpu)
      ├── meshes de chunks
      ├── mesh de entidades dinâmicas
      └── câmera como matriz pronta
```

## Responsabilidades

### `App`

- Recebe eventos do `winit`.
- Coordena atualização e renderização.
- Transfere `MeshData` pronta do jogo para o renderer.
- Não implementa geração voxel nem física.

### `Game`

- Mantém o estado da simulação.
- Decide quais chunks devem existir.
- Executa a física em passo fixo.
- Sincroniza componentes ECS com corpos do Rapier.
- Produz atualizações de renderização sem acessar a GPU.

### `VoxelWorld`

- Armazena chunks por `ChunkPos { x, y, z }`.
- Usa divisão euclidiana para coordenadas negativas.
- Faz streaming horizontal e vertical independentemente.
- Marca o chunk alterado e seus seis vizinhos como sujos.

### `Renderer`

- Recebe apenas `MeshData`, `ChunkPos` e uma matriz de câmera.
- Cria e remove buffers da GPU.
- Não conhece blocos, geração procedural, Rapier ou ECS.

### `PhysicsWorld`

- Mantém as estruturas do Rapier.
- Usa colliders trimesh estáticos para chunks.
- Cria rigid bodies dinâmicos para demonstração.

### `EcsScene`

- Mantém entidades e componentes de gameplay/renderização.
- Associa entidades a `RigidBodyHandle`.
- Converte entidades visíveis em uma mesh dinâmica temporária.

## Próximas substituições previstas

- `generate_chunk` síncrono → fila de workers.
- mesher simples → greedy meshing.
- collider trimesh → caixas agrupadas ou heightfield.
- mesh dinâmica recriada a cada frame → instancing.
- câmera livre → controlador físico do jogador.
