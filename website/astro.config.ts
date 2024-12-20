import { defineConfig } from "astro/config";

import tailwind from "@astrojs/tailwind";

import cloudflare from "@astrojs/cloudflare";

import type { AstroUserConfig } from "astro";

const config: AstroUserConfig = {
  output: "server",
  integrations: [tailwind()],
  adapter: cloudflare({
    imageService: "passthrough",
  }),
  vite: {
    build: {
      rollupOptions: {
        external: [
          "cloudflare:sockets",
        ],
      },
    },
    ssr: {
      external: [
        "node:events",
        "node:buffer",
        "node:stream",
      ],
    },
  },
};

export default defineConfig(config);
