import init from './pkg/rust_breakout.js';

async function run() {
    await init('./pkg/rust_breakout_bg.wasm');
    // Game starts automatically via #[wasm_bindgen(start)]
}

run().catch(console.error);
