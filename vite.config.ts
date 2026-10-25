import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    strictPort: true,
    host: "127.0.0.1",
    port: 1420,
    watch: {
      ignored: ["**/src-tauri/**", "**/data/**", "**/.codex/**", "**/.edge-profile/**", "**/edge-headless-profile*/**"]
    }
  },
  test: {
    environment: "node",
    globals: true
  }
});
