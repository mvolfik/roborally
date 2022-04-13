import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { join } from "node:path";

export default defineConfig({
  resolve: {
    alias: {
      "frontend-wasm": join(
        process.cwd(),
        "../backend/roborally-frontend-wasm/pkg"
      ),
    },
  },
  plugins: [svelte()],
  server: {
    fs: {
      allow: [".", "../backend/roborally-frontend-wasm/pkg"],
    },
    proxy: {
      "/websocket/": {
        ws: true,
        target: "ws://localhost:8080",
      },
      "/api/": "http://localhost:8080",
    },
  },
  build: {
    sourcemap: true,
  },
});
