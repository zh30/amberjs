import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tsconfigPaths from "vite-tsconfig-paths";
import tailwindcss from "@tailwindcss/vite";

function ssgPlugin(): Plugin {
  const root = dirname(fileURLToPath(import.meta.url));
  return {
    name: "beejs-ssg",
    apply: "build",
    closeBundle: {
      sequential: true,
      order: "post",
      handler() {
        if (process.env.BEEJS_SSR === "1") return;
        const ssr = spawnSync(
          process.execPath,
          [
            join(root, "node_modules/vite/bin/vite.js"),
            "build",
            "--ssr",
            "src/entry-server.tsx",
          ],
          {
            cwd: root,
            stdio: "inherit",
            env: { ...process.env, BEEJS_SSR: "1" },
          },
        );
        if (ssr.status !== 0) {
          throw new Error("ssr build failed");
        }
        const prerender = spawnSync(
          process.execPath,
          [join(root, "scripts/prerender.mjs")],
          { cwd: root, stdio: "inherit" },
        );
        if (prerender.status !== 0) {
          throw new Error("ssg failed");
        }
      },
    },
  };
}

export default defineConfig(({ isSsrBuild }) => ({
  plugins: [
    react(),
    tailwindcss(),
    tsconfigPaths(),
    ...(isSsrBuild ? [] : [ssgPlugin()]),
  ],
  build: {
    outDir: isSsrBuild ? "dist-ssr" : "dist",
    emptyOutDir: !isSsrBuild,
    ssr: isSsrBuild ? "src/entry-server.tsx" : undefined,
  },
  ssr: {
    noExternal: true,
    external: ["monaco-editor"],
  },
  worker: {
    format: "es",
  },
  optimizeDeps: {
    include: ["monaco-editor"],
  },
}));
