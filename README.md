# Neon Invaders

Neon Invaders is a modernized Space Invaders-style arcade shooter written in Rust with `macroquad` for Linux. It keeps the classic loop intact while adding smoother motion, colorful alien variants, destructible bunkers, particles, screen shake, a drifting starfield, local high score saving, and title / wave / game-over transitions.

## Features

- Player cannon with responsive left/right controls and a tuned fire cooldown
- Side-to-side invading formation that reverses at screen edges and steps downward
- Alien return fire that grows more intense as the formation thins out
- Destructible bunkers that degrade visibly and change the battle flow
- Score, lives, wave progression, and local high score persistence
- Procedural neon-style visuals built from shapes with no external art assets

## Build And Run On Linux

Prerequisites:

- Rust toolchain installed via `rustup`
- System packages typically needed for OpenGL/X11 builds on Linux
- ALSA development library for audio support, for example `libasound2-dev` on Debian/Ubuntu

Build and run:

```bash
cargo run
```

Run the requested checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

## Controls

- `A` / `D` or Left / Right: move
- `Space` or Up: fire
- `Space` or Enter: start / restart from menus

## High Score Save Location

The high score is stored locally at:

```text
$XDG_DATA_HOME/neon-invaders/highscore.txt
```

If `XDG_DATA_HOME` is not set, the game falls back to:

```text
~/.local/share/neon-invaders/highscore.txt
```
