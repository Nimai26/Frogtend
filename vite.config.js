import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error pas de types pour node:process sans @types/node
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [sveltekit()],

  // Réglages propres à Tauri (`tauri dev` / `tauri build`) :
  // 1. ne pas masquer les erreurs de Rust ;
  clearScreen: false,
  // 2. Tauri attend un port fixe : échouer s'il est pris ;
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. ne pas surveiller `src-tauri`.
      ignored: ["**/src-tauri/**"],
    },
  },

  // Tests de l'interface (Vitest). Les composants Svelte se testent dans un DOM simulé.
  resolve: process.env.VITEST ? { conditions: ["browser"] } : undefined,
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
}));
