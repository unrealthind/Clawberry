import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

// Only the loopback port is configurable (also used by isolated contract tests).
// The browser cannot select an upstream host or route credentials elsewhere.
const portText = process.env.RAAVICLAW_DEV_GATEWAY_PORT ?? "8080";
const port = Number(portText);
if (
  !/^\d+$/.test(portText) ||
  !Number.isInteger(port) ||
  port < 1 ||
  port > 65535
)
  throw new Error("Invalid loopback development gateway port");

export default defineConfig({
  plugins: [
    react(),
    {
      name: "raaviclaw-auth-mode",
      transformIndexHtml() {
        return [
          {
            tag: "meta",
            attrs: {
              name: "raaviclaw-auth-mode",
              content:
                process.env.VITE_GATEWAY_PAIRED === "true" ? "paired" : "auto",
            },
            injectTo: "head" as const,
          },
        ];
      },
    },
  ],
  build: {
    rollupOptions: {
      input: { app: "index.html", prototype: "prototype.html" },
    },
  },
  server: {
    proxy: { "/api": `http://127.0.0.1:${port}` },
    fs: {
      strict: true,
      allow: [fileURLToPath(new URL(".", import.meta.url))],
      deny: [
        "**/.env",
        "**/.env.*",
        "**/*.{crt,pem,key}",
        "**/.git/**",
        "**/.runtime/**",
      ],
    },
  },
});
