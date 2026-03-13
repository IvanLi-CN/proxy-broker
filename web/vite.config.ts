import path from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    chunkSizeWarningLimit: 1400,
    rollupOptions: {
      output: {
        manualChunks: {
          "react-vendor": ["react", "react-dom", "scheduler"],
          "router-vendor": ["react-router-dom"],
          "query-vendor": ["@tanstack/react-query"],
          "ui-vendor": [
            "@hookform/resolvers",
            "@radix-ui/react-checkbox",
            "@radix-ui/react-dialog",
            "@radix-ui/react-label",
            "@radix-ui/react-scroll-area",
            "@radix-ui/react-select",
            "@radix-ui/react-separator",
            "@radix-ui/react-slot",
            "@radix-ui/react-tabs",
            "@radix-ui/react-tooltip",
            "class-variance-authority",
            "lucide-react",
            "next-themes",
            "react-hook-form",
            "sonner",
            "tailwind-merge",
            "zod",
          ],
        },
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(dirname, "./src"),
    },
  },
  server: {
    host: "127.0.0.1",
    port: 38181,
    strictPort: true,
    proxy: {
      "/api": "http://127.0.0.1:8080",
      "/healthz": "http://127.0.0.1:8080",
    },
  },
  preview: {
    host: "127.0.0.1",
    port: 38183,
    strictPort: true,
  },
});
