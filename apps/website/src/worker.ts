interface Env {
  ASSETS: {
    fetch: (request: Request | string) => Promise<Response>;
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
