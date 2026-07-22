# Nexus Arena — patch multiplayer

Este ZIP é um **overlay** para o repositório `nexus-rust`.

## Aplicar

1. Crie uma branch no seu repositório:

```powershell
git checkout -b feat/multiplayer-arena
```

2. Extraia o conteúdo deste ZIP na raiz do repositório.
3. Aceite a substituição dos arquivos existentes.
4. Atualize o lockfile e valide:

```powershell
cargo fmt --all
cargo check --workspace
```

5. Faça o commit:

```powershell
git add .
git commit -m "feat: add Nexus Arena multiplayer test game"
```

## Jogar sozinho

```powershell
cargo run -p sandbox-client -- --name Fabio
```

O cliente abre um servidor integrado, igual ao modelo do Minecraft.

## Iniciar servidor dedicado

```powershell
cargo run -p sandbox-dedicated-server -- --bind 0.0.0.0:4000
```

## Conectar na mesma rede local

Descubra o IP do computador servidor:

```powershell
ipconfig
```

Se o IP for `192.168.0.10`, os clientes executam:

```powershell
cargo run -p sandbox-client -- --connect 192.168.0.10:4000 --name Amigo
```

## Jogar pela internet

Você precisa de uma destas opções:

- abrir/redirecionar a porta TCP 4000 no roteador;
- usar Tailscale;
- usar Radmin VPN;
- usar ZeroTier.

Com Tailscale/Radmin/ZeroTier, use o IP virtual do computador servidor.

Também permita o executável no Firewall do Windows.

## Regras

- W/A/S/D ou setas movimentam o cubo;
- o cubo azul é o jogador local;
- outros jogadores usam cores diferentes;
- o cubo dourado é o objetivo;
- cada coleta vale 1 ponto;
- o primeiro jogador a 5 pontos vence a rodada;
- movimento, coleta e placar são autoritativos no servidor.

## Arquitetura adicionada

- transporte TCP com mensagens enquadradas por tamanho;
- serialização `serde` + `bincode`;
- múltiplos clientes por servidor;
- servidor dedicado separado;
- servidor integrado preservado;
- broadcast de spawn, transform, despawn e placar;
- remoção automática quando uma conexão TCP cai;
- minijogo de arena autoritativo.

## Observação

O `Cargo.lock` não está incluído no overlay. Execute `cargo check --workspace`
para o Cargo adicionar o `bincode` e a nova crate ao lockfile.
