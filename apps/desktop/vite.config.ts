import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      "@projectlib/shared": resolve(__dirname, "../../packages/shared/src"),
      "@projectlib/ui": resolve(__dirname, "../../packages/ui/src"),
      "@projectlib/db": resolve(__dirname, "../../packages/db/src")
    }
  },
  server: {
    port: 5173,
    strictPort: true
  }
});
