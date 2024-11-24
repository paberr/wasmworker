// Import required functions.
import init, { runWorker, runPool, runParMap } from "./pkg/webworker_demo.js";

async function run_wasm() {
  // Load wasm bindgen.
  await init();

  console.log("index.js loaded");

  // Initialise demos.
  document.getElementById("runWorker").onclick = runWorker;
  document.getElementById("runPool").onclick = runPool;
  document.getElementById("runParMap").onclick = runParMap;
}

run_wasm();
