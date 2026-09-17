// app.config.ts
import { defineConfig } from "vite";
import { tanstackStartVite } from "@tanstack/start/vite";
import tsconfigPaths from "vite-tsconfig-paths";
var app_config_default = defineConfig({
  plugins: [
    tanstackStartVite(),
    tsconfigPaths()
  ]
});
export {
  app_config_default as default
};
