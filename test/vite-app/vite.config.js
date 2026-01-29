import { defineConfig } from 'vite';

export default defineConfig({
  // Use relative base so assets work when served from any path
  base: './',
  build: {
    // Target modern browsers that support top-level await and WASM
    target: 'esnext',
    rollupOptions: {
      // Don't try to resolve the wasm-pack output (it's a public asset)
      external: [/\.\/test\.js$/],
    },
  },
  // The wasm-pack output is placed in the public directory.
  // These files are copied to the build output as-is (not bundled).
  publicDir: 'pkg',
});
