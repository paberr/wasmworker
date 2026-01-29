// In a Vite setup, the wasm-pack output is in the public directory
// and served as static assets (not bundled by Vite).
// The @vite-ignore comment tells Vite not to resolve this dynamic import.
const { default: init, runTests } = await import(/* @vite-ignore */ './test.js');

// Init wasm bindgen.
await init();

// Run tests defined in Rust.
await runTests();

// Call onDone to notify test runner.
onDone();
