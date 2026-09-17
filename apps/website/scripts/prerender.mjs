/**
 * After Vite writes dist/index.html, emit one HTML file per route so
 * view-source and crawlers see real tags. The client still hydrates via
 * createRoot and replaces #root after JS loads.
 */
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");
const DIST = join(ROOT, "dist");
const SITE = "https://amberjs.com";

function esc(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function parseFrontmatter(raw) {
  const match = raw.match(/^---\s*([\s\S]*?)\s*---\s*([\s\S]*)$/);
  if (!match) return { data: {}, body: raw };
  const data = {};
  for (const line of match[1].split("\n")) {
    const i = line.indexOf(":");
    if (i === -1) continue;
    const key = line.slice(0, i).trim();
    let value = line.slice(i + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    data[key] = value;
  }
  return { data, body: match[2] };
}

function inline(text) {
  let s = esc(text);
  s = s.replace(/`([^`]+)`/g, "<code>$1</code>");
  s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
  return s;
}

function mdToHtml(md) {
  const lines = md.replace(/\r\n/g, "\n").split("\n");
  const out = [];
  let i = 0;
  const flushPara = (buf) => {
    const t = buf.join(" ").trim();
    if (t) out.push(`<p>${inline(t)}</p>`);
    buf.length = 0;
  };

  while (i < lines.length) {
    const line = lines[i];

    if (line.startsWith("```")) {
      const lang = esc(line.slice(3).trim());
      i += 1;
      const code = [];
      while (i < lines.length && !lines[i].startsWith("```")) {
        code.push(lines[i]);
        i += 1;
      }
      i += 1;
      out.push(
        `<pre><code${lang ? ` class="language-${lang}"` : ""}>${esc(code.join("\n"))}</code></pre>`,
      );
      continue;
    }

    const h = line.match(/^(#{1,6})\s+(.+)$/);
    if (h) {
      const level = h[1].length;
      out.push(`<h${level}>${inline(h[2])}</h${level}>`);
      i += 1;
      continue;
    }

    if (line.startsWith("> ")) {
      const q = [];
      while (i < lines.length && lines[i].startsWith("> ")) {
        q.push(lines[i].slice(2));
        i += 1;
      }
      out.push(`<blockquote>${inline(q.join(" "))}</blockquote>`);
      continue;
    }

    if (
      /^\|/.test(line) &&
      i + 1 < lines.length &&
      /^\|?\s*-/.test(lines[i + 1])
    ) {
      const rows = [];
      while (i < lines.length && /^\|/.test(lines[i])) {
        if (!/^\|?\s*-/.test(lines[i])) {
          rows.push(
            lines[i]
              .split("|")
              .slice(1, -1)
              .map((c) => c.trim()),
          );
        }
        i += 1;
      }
      if (rows.length) {
        const head = rows[0];
        const body = rows.slice(1);
        out.push("<table><thead><tr>");
        for (const c of head) out.push(`<th>${inline(c)}</th>`);
        out.push("</tr></thead><tbody>");
        for (const row of body) {
          out.push("<tr>");
          for (const c of row) out.push(`<td>${inline(c)}</td>`);
          out.push("</tr>");
        }
        out.push("</tbody></table>");
      }
      continue;
    }

    if (/^[-*]\s+/.test(line)) {
      out.push("<ul>");
      while (i < lines.length && /^[-*]\s+/.test(lines[i])) {
        out.push(`<li>${inline(lines[i].replace(/^[-*]\s+/, ""))}</li>`);
        i += 1;
      }
      out.push("</ul>");
      continue;
    }

    if (/^\d+\.\s+/.test(line)) {
      out.push("<ol>");
      while (i < lines.length && /^\d+\.\s+/.test(lines[i])) {
        out.push(`<li>${inline(lines[i].replace(/^\d+\.\s+/, ""))}</li>`);
        i += 1;
      }
      out.push("</ol>");
      continue;
    }

    if (!line.trim()) {
      i += 1;
      continue;
    }

    const para = [];
    while (
      i < lines.length &&
      lines[i].trim() &&
      !/^(#{1,6}\s|```|[\-*]\s|\d+\.\s|\|)/.test(lines[i])
    ) {
      para.push(lines[i]);
      i += 1;
    }
    if (para.length) flushPara(para);
    else i += 1;
  }

  return out.join("\n");
}

function listEnglishMarkdown(dir) {
  return readdirSync(dir)
    .filter((f) => f.endsWith(".md") && !/\.[a-z]{2}\.md$/.test(f))
    .sort();
}

function setMeta(html, attr, value) {
  const escaped = esc(value);
  const reName = new RegExp(
    `(<meta\\s+name="${attr}"\\s+content=")[^"]*(")`,
    "i",
  );
  const reProp = new RegExp(
    `(<meta\\s+property="${attr}"\\s+content=")[^"]*(")`,
    "i",
  );
  if (reName.test(html)) html = html.replace(reName, `$1${escaped}$2`);
  if (reProp.test(html)) html = html.replace(reProp, `$1${escaped}$2`);
  return html;
}

function excerpt(md, fallback) {
  const text = String(md)
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/[#>*_`\[\]|]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!text) return fallback;
  if (text.length <= 155) return text;
  return `${text.slice(0, 152).replace(/\s+\S*$/, "")}…`;
}

function applyHead(
  template,
  { title, description, canonical, jsonLd, ogType },
) {
  let html = template;
  html = html.replace(/<title>[^<]*<\/title>/, `<title>${esc(title)}</title>`);
  html = html.replace(
    /<link rel="canonical" href="[^"]*" \/>/,
    `<link rel="canonical" href="${esc(canonical)}" />`,
  );
  html = setMeta(html, "description", description);
  html = setMeta(html, "og:type", ogType || "website");
  html = setMeta(html, "og:title", title);
  html = setMeta(html, "og:description", description);
  html = setMeta(html, "og:url", canonical);
  html = setMeta(html, "twitter:title", title);
  html = setMeta(html, "twitter:description", description);
  html = setMeta(html, "twitter:url", canonical);
  if (jsonLd) {
    html = html.replace(
      /<script type="application\/ld\+json">[\s\S]*?<\/script>/,
      `<script type="application/ld+json">\n${jsonLd}\n  </script>`,
    );
  }
  return html;
}

function injectRoot(html, body) {
  const start = html.indexOf('<div id="root">');
  const noscript = html.indexOf("<noscript>", start);
  if (start === -1 || noscript === -1) {
    throw new Error("dist/index.html is missing #root or noscript");
  }
  return `${html.slice(0, start)}<div id="root">${body}</div>\n  ${html.slice(noscript)}`;
}

function writePage(relPath, html) {
  const file = join(DIST, relPath);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, html);
}

function nav() {
  return `<nav><a href="/">Amber</a> · <a href="/docs">Docs</a> · <a href="/blog">Blog</a> · <a href="https://github.com/zh30/amberjs">GitHub</a></nav>`;
}

function loadDocs() {
  const docsDir = join(ROOT, "src/docs");
  return listEnglishMarkdown(docsDir).map((file) => {
    const raw = readFileSync(join(docsDir, file), "utf8");
    const { data, body } = parseFrontmatter(raw);
    const id = data.id || file.replace(/\.md$/, "");
    return {
      id,
      title: data.title || id,
      subtitle: data.subtitle || "",
      body,
    };
  });
}

function loadPosts() {
  const blogDir = join(ROOT, "src/blog");
  return listEnglishMarkdown(blogDir).map((file) => {
    const raw = readFileSync(join(blogDir, file), "utf8");
    const { data, body } = parseFrontmatter(raw);
    const slug = file.replace(/\.md$/, "");
    return {
      slug,
      title: data.title || slug,
      excerpt: data.excerpt || data.title || slug,
      date: data.date || "",
      tag: data.tag || "Blog",
      author: data.author || "Amber",
      body,
    };
  });
}

export function renderRoute(pathname) {
  const path =
    decodeURIComponent((pathname || "/").split("?")[0].split("#")[0]).replace(
      /\/+$/,
      "",
    ) || "/";

  const homeTitle = "Amber | JavaScript and TypeScript runtime in Rust and V8";
  const homeDesc =
    "Amber is a JS/TS runtime in Rust and V8. One amber binary for scripts, Jest-style tests, MCP tools, and an opt-in capability sandbox. Not a Node.js clone.";

  if (path === "/") {
    return {
      title: homeTitle,
      description: homeDesc,
      canonical: `${SITE}/`,
      body: `<main>
${nav()}
<p>v1.16.0</p>
<h1>A JavaScript &amp; TypeScript runtime in Rust &amp; V8</h1>
<p>${esc(homeDesc)}</p>
<p><a href="/docs">Explore Docs</a> · <a href="/docs/installation">Installation</a></p>
<pre><code>curl -fsSL https://amberjs.com/install.sh | sh</code></pre>
</main>`,
    };
  }

  const docs = loadDocs();
  if (path === "/docs") {
    return {
      title: "Docs | Amber",
      description:
        "Amber manual: install bee, CLI, capability sandbox, Node and Web APIs, and amber:ai. Per-API coverage, not a Node.js clone.",
      canonical: `${SITE}/docs`,
      body: `<main>
${nav()}
<h1>Amber Docs</h1>
<ul>
${docs.map((d) => `<li><a href="/docs/${esc(d.id)}">${esc(d.title)}</a></li>`).join("\n")}
</ul>
</main>`,
    };
  }
  if (path.startsWith("/docs/")) {
    const id = path.slice("/docs/".length);
    const doc = docs.find((d) => d.id === id);
    if (doc) {
      return {
        title: `${doc.title} | Amber Docs`,
        description: excerpt(
          doc.body,
          doc.subtitle || `${doc.title} in the Beejs runtime manual.`,
        ),
        canonical: `${SITE}/docs/${doc.id}`,
        body: `<main>
${nav()}
<article>
<h1>${esc(doc.title)}</h1>
${doc.subtitle ? `<p>${esc(doc.subtitle)}</p>` : ""}
${mdToHtml(doc.body)}
</article>
</main>`,
      };
    }
  }

  const posts = loadPosts();
  if (path === "/blog") {
    return {
      title: "Blog | Amber",
      description:
        "Amber release notes: runtime changes, Wasm, packaging, and performance work.",
      canonical: `${SITE}/blog`,
      body: `<main>
${nav()}
<h1>Release notes</h1>
<ul>
${posts.map((p) => `<li><a href="/blog/${esc(p.slug)}">${esc(p.title)}</a></li>`).join("\n")}
</ul>
</main>`,
    };
  }
  if (path.startsWith("/blog/")) {
    const slug = path.slice("/blog/".length);
    const post = posts.find((p) => p.slug === slug);
    if (post) {
      return {
        title: `${post.title} | Amber`,
        description: excerpt(post.excerpt || post.body, post.title),
        ogType: "article",
        canonical: `${SITE}/blog/${post.slug}`,
        jsonLd: JSON.stringify({
          "@context": "https://schema.org",
          "@type": "BlogPosting",
          headline: post.title,
          description: post.excerpt,
          datePublished: post.date || undefined,
          author: { "@type": "Organization", name: post.author },
          url: `${SITE}/blog/${post.slug}`,
        }),
        body: `<main>
${nav()}
<article>
<h1>${esc(post.title)}</h1>
${mdToHtml(post.body)}
</article>
</main>`,
      };
    }
  }
  if (path === "/play") {
    return {
      title: "Playground | Amber",
      description:
        "Write JavaScript and TypeScript in Monaco (the VS Code editor) and run it in the browser. Not the amber binary — Amber cannot ship V8 inside WASM.",
      canonical: `${SITE}/play`,
      body: `<main><h1>Playground</h1><p>Browser JS/TS editor. Loading…</p></main>`,
    };
  }
  return null;
}

export function applyPage(template, page) {
  if (!page) return template;
  return injectRoot(applyHead(template, page), page.body);
}

export async function prerender() {
  const ssrUrl = pathToFileURL(join(ROOT, "dist-ssr/entry-server.js")).href;
  const { render } = await import(ssrUrl);
  const template = readFileSync(join(DIST, "index.html"), "utf8");
  const docs = loadDocs();
  const posts = loadPosts();
  const urls = [
    "/",
    "/docs",
    "/blog",
    "/play",
    ...docs.map((d) => `/docs/${d.id}`),
    ...posts.map((p) => `/blog/${p.slug}`),
  ];
  for (const path of urls) {
    const page = renderRoute(path);
    if (!page) continue;
    const appHtml = render(path);
    const html = injectRoot(applyHead(template, page), appHtml);
    if (path === "/") {
      writeFileSync(join(DIST, "index.html"), html);
      continue;
    }
    writePage(`${path.slice(1)}/index.html`, html);
    writePage(`${path.slice(1)}.html`, html);
  }
  const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls.map((u) => `  <url>\n    <loc>${SITE}${u}</loc>\n  </url>`).join("\n")}
</urlset>\n`;
  writeFileSync(join(DIST, "sitemap.xml"), sitemap);
  console.log(`ssg ${urls.length} routes`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  await prerender();
}
