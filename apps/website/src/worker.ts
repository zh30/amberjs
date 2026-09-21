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

    const trimmed = path.replace(/\/+$/, "") || "/";
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
