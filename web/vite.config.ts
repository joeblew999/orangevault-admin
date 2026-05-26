import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// During `vite dev`, proxy ConnectRPC + /healthz to a locally-running
// `wrangler dev` so the SPA can talk to the worker exactly as it will
// in production (where the worker serves both the assets and the API).
// Default to the deployed worker so `npm run dev` works without a
// local `wrangler dev` running. Override via VITE_WORKER_ORIGIN env to
// hit a local wrangler dev (e.g. VITE_WORKER_ORIGIN=http://127.0.0.1:8787).
const WORKER_ORIGIN =
  process.env.VITE_WORKER_ORIGIN ?? "https://orangevault-admin.gedw99.workers.dev";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5174,
    proxy: {
      "^/orangevault_admin\\.": { target: WORKER_ORIGIN, changeOrigin: true, secure: false },
      "^/healthz$": { target: WORKER_ORIGIN, changeOrigin: true, secure: false },
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
