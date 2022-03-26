import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
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
});
