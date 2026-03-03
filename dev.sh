#!/bin/bash
# Dev mode: watches Rust source → rebuilds WASM → Vite hot-reloads
set -e

echo "🦀 Starting Rust Breakout dev mode..."
echo "   Vite on http://localhost:3011"
echo "   Watching src/ for Rust changes → auto-rebuild WASM"
echo ""

# Start Vite in background
cd www && npx vite --port 3011 --host &
VITE_PID=$!
cd ..

# Watch Rust source and rebuild on change
cargo watch -w src -s "wasm-pack build --dev --target web --out-dir www/pkg 2>&1 | tail -1 && echo '✅ WASM rebuilt'" &
WATCH_PID=$!

trap "kill $VITE_PID $WATCH_PID 2>/dev/null; exit" INT TERM

wait
