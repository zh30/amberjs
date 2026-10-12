import { LEGACY_DOC_REDIRECTS } from "./lib/docs-nav";

interface Env {
  ASSETS: {
    fetch: (request: Request | string) => Promise<Response>;
  };
  DIST_BUCKET?: {
    get: (key: string) => Promise<{
      body: ReadableStream;
      httpEtag: string;
    } | null>;
  };
}

async function firstOk(
  env: Env,
  origin: string,
  paths: string[],
): Promise<Response | null> {
  for (const path of paths) {
    const res = await env.ASSETS.fetch(new Request(new URL(path, origin)));
    if (res.ok) return res;
  }
  return null;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (!env.ASSETS) {
      return new Response("ASSETS binding is not configured", { status: 500 });
    }

    let url: URL;
    try {
      url = new URL(request.url);
    } catch {
      return new Response("Bad Request", { status: 400 });
    }
    const path = url.pathname;
    const isGetSubdomain = url.hostname === "get.amberjs.com";

    // Handle R2 install scripts (for get.amberjs.com or /install.sh / /install.ps1)
    if (isGetSubdomain || path === "/install.sh" || path === "/install.ps1") {
      let objectKey = "";
      if (path === "/install.ps1") {
        objectKey = "install.ps1";
      } else if (path === "/install.sh" || (isGetSubdomain && (path === "/" || path === ""))) {
        objectKey = "install.sh";
      }

      if (objectKey && env.DIST_BUCKET) {
        try {
          const object = await env.DIST_BUCKET.get(objectKey);
          if (object) {
            const headers = new Headers();
            headers.set("content-type", "text/plain; charset=utf-8");
            headers.set("cache-control", "public, max-age=300");
            headers.set("etag", object.httpEtag);
            headers.set("access-control-allow-origin", "*");
            return new Response(object.body, { headers });
          }
        } catch {
          // fallback to assets if R2 fetch fails
        }
      }
    }

    if (
      path === "/robots.txt" ||
      path === "/sitemap.xml" ||
      path === "/favicon.ico" ||
      path.startsWith("/.")
    ) {
      return env.ASSETS.fetch(request);
    }

    if (path === "/llms.txt" || path === "/llms-full.txt") {
      const res = await env.ASSETS.fetch(request);
      const headers = new Headers(res.headers);
      headers.set("content-type", "text/plain; charset=utf-8");
      headers.set("access-control-allow-origin", "*");
      return new Response(res.body, { status: res.status, headers });
    }

    if (path.endsWith(".md")) {
      const res = await env.ASSETS.fetch(request);
      const type = res.headers.get("content-type") || "";
      if (!res.ok || type.startsWith("text/html")) {
        return new Response(
          `Not found: ${path}\nIndex of agent-readable pages: ${url.origin}/llms.txt\n`,
          { status: 404, headers: { "content-type": "text/plain; charset=utf-8" } },
        );
      }
      const headers = new Headers(res.headers);
      headers.set("content-type", "text/markdown; charset=utf-8");
      headers.set("access-control-allow-origin", "*");
      const html = path === "/index.md" ? "/" : path.slice(0, -".md".length);
      headers.set("link", `<${url.origin}${html}>; rel="canonical"`);
      return new Response(res.body, { status: res.status, headers });
    }

    const trimmed = path.replace(/\/+$/, "") || "/";
    const legacyDoc = trimmed.match(/^\/docs\/([^/]+)$/);
    if (legacyDoc && Object.hasOwn(LEGACY_DOC_REDIRECTS, legacyDoc[1])) {
      const target = new URL(`/docs/${LEGACY_DOC_REDIRECTS[legacyDoc[1]]}`, url.origin);
      target.search = url.search;
      return Response.redirect(target.toString(), 301);
    }
    if (trimmed !== "/") {
      const prerendered = await firstOk(env, url.origin, [
        `${trimmed}.html`,
        `${trimmed}/index.html`,
      ]);
      if (prerendered) return prerendered;
    }

    const assetResponse = await env.ASSETS.fetch(request);
    if (assetResponse.status !== 404) {
      return assetResponse;
    }

    const indexRequest = new Request(new URL("/index.html", url.origin), {
      method: "GET",
      headers: request.headers,
    });
    return env.ASSETS.fetch(indexRequest);
  },
};
