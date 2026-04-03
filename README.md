# Neon Invaders

Neon Invaders is a modernized Space Invaders-style arcade shooter written in Rust with `macroquad`. It keeps the classic loop intact while adding smoother motion, colorful alien variants, destructible bunkers, particles, screen shake, a drifting starfield, local profile saving, and title / wave / game-over transitions.

## Features

- Player cannon with responsive left/right controls and a tuned fire cooldown
- Side-to-side invading formation that reverses at screen edges and steps downward
- Alien return fire that grows more intense as the formation thins out
- Destructible bunkers that degrade visibly and change the battle flow
- Score, lives, wave progression, local player profiles, and persistent high scores
- Procedural neon-style visuals built from shapes with no external art assets
- Linux-native development flow with Windows cross-compilation support from Linux

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

## Cross-Compile For Windows From Linux

Install the Windows GNU target and MinGW cross toolchain:

```bash
rustup target add x86_64-pc-windows-gnu
```

On Debian / Ubuntu:

```bash
sudo apt install mingw-w64
```

Then build a Windows executable from Linux:

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

The project includes `.cargo/config.toml` for the common MinGW linker:

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
```

Expected output:

```text
target/x86_64-pc-windows-gnu/release/invaders.exe
```

Cross-compiling is enough to produce a Windows build, but you should still test the final `.exe` on a real Windows machine or Windows CI runner for runtime validation.

## Remote Score Server

A separate authenticated score backend scaffold lives in:

```text
score-server/
```

It provides:

- email magic-link login
- access and refresh tokens
- leaderboard reads
- authenticated score submission

See [score-server/README.md](score-server/README.md) for setup and API examples.

## Controls

- `A` / `D` or Left / Right: move
- `Space`: fire
- `Up`: launch player bomb
- `Esc`: quit from title, pause in-game, return to title from pause / game over
- `Space` or `Enter`: start / restart from menus
- `N`: add a new player on the title screen
- `Up` / `Down`: select an existing player on the title screen

## Profile Save Location

Linux:

```text
$XDG_DATA_HOME/neon-invaders/profiles.txt
```

If `XDG_DATA_HOME` is not set, the game falls back to:

```text
~/.local/share/neon-invaders/profiles.txt
```

Windows:

```text
%APPDATA%\Neon Invaders\profiles.txt
```
