import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  server: {
    fs: {
      allow: [".", "../backend/roborally-frontend-wasm/pkg"],
    },
    proxy: {
      "/game": {
        ws: true,
        target: "ws://localhost:8080/",
      },
      "/new-game": "http://localhost:8080/",
      "/list-games": "http://localhost:8080/",
    },
  },
});
