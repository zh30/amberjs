/* eslint-disable react-refresh/only-export-components -- SSR-only entry, never hot-reloaded; it also exports build data for scripts/agent-docs.mjs. */
import { renderToString } from "react-dom/server";
import { StaticRouter } from "react-router-dom";
import { AppRoutes } from "./app";
import { en } from "./lib/locales/en";

export function render(url: string): string {
  return renderToString(
    <StaticRouter location={url}>
      <AppRoutes />
    </StaticRouter>,
  );
}

// Build-time data for scripts/agent-docs.mjs (llms.txt and .md pages).
export * as scope from "./lib/scope";
export { DOC_NAV } from "./lib/docs-nav";
export { AMBER_VERSION } from "./lib/version";
export const docGroups = en.docs.groups;
