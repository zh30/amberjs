import type { DocGroupKey } from "./locales/types";

export const DOC_NAV: readonly { group: DocGroupKey; items: readonly string[] }[] =
  [
    { group: "start", items: ["introduction", "installation", "quick-start"] },
    {
      group: "guides",
      items: [
        "cli-usage",
        "typescript",
        "testing",
        "server-mode",
        "package-manager",
        "bundling-compilation",
        "debugging",
      ],
    },
    {
      group: "compat",
      items: ["stability", "node-compat", "web-apis", "wintertc-compliance"],
    },
    {
      group: "agents",
      items: ["agent-sandbox", "mcp-protocol", "ai-engine", "wasm-interop"],
    },
    { group: "experimental", items: ["experimental"] },
  ];

/**
 * Pages retired in the v1.17 docs rewrite. Shared by the docs route and the
 * Cloudflare worker so old links land on the closest current page.
 */
export const LEGACY_DOC_REDIRECTS: Readonly<Record<string, string>> = {
  "jit-optimization": "typescript",
  "types-lsp": "typescript",
  "testing-benchmarking": "testing",
  "debugging-lsp": "debugging",
  "package-manager-dlx": "package-manager",
  "v8-isolate-pool": "introduction",
  "memory-management": "node-compat",
  "framework-compat": "node-compat",
  "api-reference": "stability",
  modules: "node-compat",
  "hardened-sandbox": "agent-sandbox",
  "capability-security": "agent-sandbox",
  "virtual-fs-sandbox": "experimental",
  "isolate-pool": "experimental",
  "ai-embeddings": "ai-engine",
  "slm-inference": "ai-engine",
  "embedded-db": "experimental",
  "standard-library": "experimental",
  "deployment-docker": "experimental",
  "ide-extension": "experimental",
  "task-runner": "experimental",
  "code-quality": "experimental",
  "agent-replay": "experimental",
  "model-weights": "experimental",
  "kv-store": "experimental",
  "tool-synthesis": "experimental",
  "agent-bus": "experimental",
  "streaming-grammar": "experimental",
  "agent-checkpoint": "experimental",
  "ffi-native": "experimental",
  "import-maps-native": "experimental",
};
