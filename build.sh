#!/bin/bash
set -e

echo "🔨 Building WASM with wasm-pack..."
wasm-pack build --target web --out-dir www/pkg

echo "✅ Build complete!"
echo ""
echo "To play, serve the www/ directory:"
echo "  cd www && python3 -m http.server 8080"
echo "  Then open http://localhost:8080"
