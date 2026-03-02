# Rust Breakout 🧱

A classic Breakout/brick-breaker game written in pure Rust, compiled to WebAssembly, and playable in the browser.

![Neon Breakout](https://img.shields.io/badge/Rust-WASM-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- 🎮 Classic Breakout gameplay with smooth physics
- 🌈 6 rows of neon-colored bricks (pink, orange, yellow, green, cyan, purple)
- ✨ Particle effects when bricks break
- 🏃 Ball speed increases as you destroy bricks
- 🖱️ Mouse AND keyboard (arrow keys / WASD) paddle control
- 💫 Ball trail, glow effects, dark neon aesthetic
- 📊 Score, lives, and speed HUD
- 🎯 Game over / win screens with restart

## Tech Stack

- **Rust** with `wasm-bindgen` + `web-sys`
- **HTML5 Canvas 2D** (no WebGL, no game engine)
- **wasm-pack** for building

## Build & Run

### Prerequisites

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### Build

```bash
chmod +x build.sh
./build.sh
```

### Play

```bash
cd www
python3 -m http.server 8080
```

Open [http://localhost:8080](http://localhost:8080) in your browser.

## Controls

| Input | Action |
|-------|--------|
| Mouse | Move paddle |
| ← → / A D | Move paddle (keyboard) |
| Click / Space | Start game / Launch ball |
| Space | Restart after game over |

## Project Structure

```
rust-breakout/
├── Cargo.toml          # Rust dependencies
├── src/
│   └── lib.rs          # All game logic (~500 lines)
├── www/
│   ├── index.html      # Host page
│   ├── index.js        # WASM loader
│   └── pkg/            # Built WASM output (generated)
├── build.sh            # Build script
└── README.md
```

## License

MIT — built by [Big Idea Games](https://github.com/bigideagames)
