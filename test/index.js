import init, { runTests } from './pkg/test.js';

// Init wasm bindgen.
await init();

// Run tests defined in Rust.
await runTests();

// Call onDone to notify test runner.
onDone();
