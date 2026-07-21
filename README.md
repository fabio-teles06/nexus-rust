# Nexus Voxel Prototype

Protótipo de engine voxel em Rust sem usar uma engine pronta.

## Incluído

- Renderização com `wgpu`.
- Janela e input com `winit`.
- Chunks tridimensionais de `16 × 16 × 16`, inclusive no eixo vertical.
- Streaming de chunks em torno da câmera.
- Geração procedural simples de terreno.
- Mesh por chunk, desenhando apenas faces expostas.
- Física com Rapier 3D.
- Collider trimesh por chunk.
- Caixas físicas dinâmicas.
- ECS standalone com `bevy_ecs`.
- GUI de debug com `egui`.

## Requisitos

Este projeto usa crates atuais e requer Rust 1.95 ou mais recente:

```powershell
rustup update stable
```

## Executar

```powershell
cargo run
```

A primeira compilação pode levar alguns minutos por causa do `wgpu`, Rapier e egui.

Para validar antes de executar:

```powershell
cargo fmt --check
cargo test
cargo check
```

## Controles

| Controle | Ação |
|---|---|
| W A S D | Mover câmera |
| Espaço / Ctrl | Subir / descer |
| Shift | Acelerar |
| Mouse | Olhar |
| F1 | Abrir ou fechar painel de debug |
| Esc | Liberar mouse; novamente fecha |
| Clique esquerdo | Capturar mouse quando estiver livre |

## Estrutura

```text
src/
├── app.rs              # Event loop e coordenação
├── camera.rs           # Câmera livre
├── debug_gui.rs        # egui + painel de debug
├── ecs_scene.rs        # Componentes e entidades
├── game.rs             # Estado e regras do protótipo
├── input.rs            # Estado do teclado e mouse
├── mesh.rs             # Formato genérico de mesh
├── physics.rs          # Rapier
├── renderer.rs         # wgpu e buffers GPU
├── shader.wgsl         # Shader do mundo
└── voxel/
    ├── block.rs
    ├── chunk.rs
    ├── generator.rs
    ├── mesher.rs
    └── world.rs
```

## Limitações intencionais do MVP

- Geração e meshing ainda são síncronos.
- O mesher ainda não usa greedy meshing.
- Colliders usam a mesma superfície triangular da mesh visual.
- A câmera é livre e não representa ainda um jogador físico.
- Não há persistência do mundo.

A arquitetura separa essas áreas para permitir substituir cada implementação sem acoplar o renderer ao mundo. Consulte também [`ARCHITECTURE.md`](ARCHITECTURE.md).
