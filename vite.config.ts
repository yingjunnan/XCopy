import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  // Multiple HTML entry points: the main clipboard popup and the image preview
  // window. Tauri serves these from the frontend dist root at runtime.
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        preview: resolve(__dirname, "preview.html"),
        quickPaste: resolve(__dirname, "quick_paste.html"),
        onboarding: resolve(__dirname, "onboarding.html"),
      },
    },
  },
}));

