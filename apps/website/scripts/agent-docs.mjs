/**
 * Agent-readable Markdown for amberjs.com (llms.txt, llms-full.txt, per-page .md).
 * Everything is derived at build time from src/docs/*.md, src/lib/scope.ts
 * (mirror of docs/CURRENT_SCOPE.md), tests/conformance/scorecard.md and LICENSE,
 * so the Markdown cannot drift from the HTML site.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const SITE = "https://amberjs.com";

/** HTML route → its Markdown twin. Keep in sync with the worker's .md handling. */
export function markdownPathFor(route) {
  if (route === "/") return "/index.md";
  return `${route}.md`;
}

function md(route) {
  return `${SITE}${markdownPathFor(route)}`;
}

/** Point site-relative links at the Markdown twins so agents stay in .md land. */
function rewriteLinks(body, knownRoutes) {
  return body.replace(/\]\((\/[^)\s#]*)(#[^)\s]*)?\)/g, (whole, path, hash) => {
    const route = path.replace(/\/+$/, "") || "/";
    if (!knownRoutes.has(route)) return `](${SITE}${path}${hash || ""})`;
    return `](${md(route)}${hash || ""})`;
  });
}

function readScorecard(repoRoot, fallback) {
  const file = join(repoRoot, "tests/conformance/scorecard.md");
  if (!existsSync(file)) return { ...fallback, source: "src/lib/scope.ts" };
  const m = readFileSync(file, "utf8").match(/Pass rate:\s*(\d+)\/(\d+)/);
  if (!m) return { ...fallback, source: "src/lib/scope.ts" };
  const pass = Number(m[1]);
  const total = Number(m[2]);
  if (pass !== fallback.pass || total !== fallback.total) {
    console.warn(
      `agent-docs: scorecard.md says ${pass}/${total} but src/lib/scope.ts CONFORMANCE says ${fallback.pass}/${fallback.total}; using scorecard.md`,
    );
  }
  return { pass, total, gate: fallback.gate, source: "tests/conformance/scorecard.md" };
}

function readLicense(repoRoot) {
  const file = join(repoRoot, "LICENSE");
  if (!existsSync(file)) return "MIT";
  const first = readFileSync(file, "utf8").split("\n")[0].trim();
  return first.replace(/\s+License$/i, "") || "MIT";
}

function page({ title, summary, htmlRoute, body }) {
  const lines = [`# ${title}`, ""];
  if (summary) lines.push(`> ${summary}`, "");
  lines.push(
    `HTML: ${SITE}${htmlRoute === "/" ? "/" : htmlRoute} · Index for agents: ${SITE}/llms.txt`,
    "",
    body.trim(),
    "",
  );
  return lines.join("\n");
}

function stableLabel(item) {
  const id = item.g
    ? `${item.g}${item.qualifier ? ` (${item.qualifier})` : ""}`
    : "base scope";
  return id;
}

function escCell(s) {
  return String(s).replace(/\|/g, "\\|").replace(/\n/g, " ");
}

function compatibilityBody(scope, conformance, contractUrl) {
  const areaTitle = {
    cli: "CLI & tooling",
    runtime: "Runtime",
    node: "Node.js APIs",
    web: "Web APIs",
  };
  const out = [];
  out.push(
    `Source of truth: [docs/CURRENT_SCOPE.md](${scope.SCOPE_DOC_URL}). If this page and that file disagree, the file wins.`,
    "",
    `- Stable contracts: G1–G${scope.GRADUATED_CONTRACTS} (some rows group several ids; some are marked tentative or provisional).`,
    `- Node conformance scorecard: **${conformance.pass}/${conformance.total} PASS** ([scorecard](${scope.CONFORMANCE_URL})). CI fails if fewer than ${conformance.gate} pass or any fixture fails or is skipped. A passing fixture is a regression gate; it does not by itself make an API Stable.`,
    "- Performance: Amber publishes no current performance numbers. Older benchmark tables in the repository are historical.",
    "",
    "## Stable",
    "",
    "Pinned by a contract document and an executable CI test. Limits are listed next to each API; anything not listed is not promised.",
    "",
  );
  for (const area of ["cli", "runtime", "node", "web"]) {
    const rows = scope.STABLE.filter((s) => s.area === area);
    if (!rows.length) continue;
    out.push(`### ${areaTitle[area]}`, "", "| Capability | API | Id | Limits | Contract |", "| :--- | :--- | :--- | :--- | :--- |");
    for (const s of rows) {
      const contract = s.doc ? `[${s.doc}.md](${contractUrl(s.doc)})` : "—";
      out.push(
        `| ${escCell(s.name)} | \`${escCell(s.api)}\` | ${stableLabel(s)} | ${escCell(s.limits.en)} | ${contract} |`,
      );
    }
    out.push("");
  }
  const list = (title, intro, items) => {
    out.push(`## ${title}`, "", intro, "");
    for (const i of items) out.push(`- **${i.name}** — ${i.detail.en}`);
    out.push("");
  };
  list(
    "Preview",
    "In the default build and usable, but the contract or edge cases are still being tightened.",
    scope.PREVIEW,
  );
  list(
    "Experimental",
    "Code exists, but it is not a product promise until it is re-verified.",
    scope.EXPERIMENTAL,
  );
  out.push(
    "### Present in the binary but not classified by CURRENT_SCOPE (treated as Experimental)",
    "",
    scope.UNCLASSIFIED.map((u) => `\`${u}\``).join(", "),
    "",
  );
  list(
    "Not supported (non-goals)",
    "Deliberately out of scope today. Where possible Amber throws or rejects instead of returning fake data.",
    scope.NON_GOALS,
  );
  return out.join("\n");
}

function factsBlock({ version, license, conformance, scope }) {
  return [
    "## Key facts",
    "",
    `- Current release: ${version}. Install: \`curl -fsSL https://get.amberjs.com/install.sh | sh\` (macOS/Linux) or \`irm https://get.amberjs.com/install.ps1 | iex\` (Windows PowerShell). See [Installation](${md("/docs/installation")}).`,
    `- License: ${license}, free and open source. Source: ${scope.REPO_URL}`,
    "- Pricing: none. Amber is free to use; there is no paid tier, plan, or license fee.",
    `- Status labels: Stable / Preview / Experimental / Not supported, from [docs/CURRENT_SCOPE.md](${scope.SCOPE_DOC_URL}). ${scope.GRADUATED_CONTRACTS} Stable contracts (G1–G${scope.GRADUATED_CONTRACTS}). Full table: [Compatibility](${md("/compatibility")}).`,
    `- Node conformance scorecard: ${conformance.pass}/${conformance.total} PASS on main.`,
    "- Amber is not a drop-in replacement for Node.js, Bun, or Deno, and publishes no current performance numbers.",
    "",
  ].join("\n");
}

export function writeAgentDocs({ dist, repoRoot, docs, posts, ssr }) {
  const { scope, DOC_NAV, AMBER_VERSION, docGroups } = ssr;
  const version = AMBER_VERSION;
  const license = readLicense(repoRoot);
  const conformance = readScorecard(repoRoot, scope.CONFORMANCE);
  const byId = new Map(docs.map((d) => [d.id, d]));
  const knownRoutes = new Set(["/", "/compatibility", "/docs", ...docs.map((d) => `/docs/${d.id}`)]);
  const facts = factsBlock({ version, license, conformance, scope });
  const summary = `Amber is a free, open-source (${license}) JavaScript and TypeScript runtime built with Rust and V8. One \`amber\` binary runs scripts, Jest-style tests, bundling, single-file compile, a small package installer, an HTTP serve command, and an opt-in capability sandbox for agent tools. It is not a drop-in Node.js replacement: each API is labeled Stable, Preview, Experimental, or Not supported.`;

  const navDocs = DOC_NAV.flatMap((g) => g.items.map((id) => ({ group: g.group, doc: byId.get(id) }))).filter((x) => x.doc);
  const missing = docs.filter((d) => !navDocs.some((n) => n.doc.id === d.id));
  for (const d of missing) navDocs.push({ group: "other", doc: d });

  const files = new Map();

  const intro = byId.get("introduction");
  files.set(
    "/",
    page({
      title: `Amber ${version} — JavaScript and TypeScript runtime in Rust and V8`,
      summary,
      htmlRoute: "/",
      body: [
        facts,
        intro ? rewriteLinks(intro.body, knownRoutes) : "",
        "",
        "## Pages",
        "",
        `- [Compatibility & scope](${md("/compatibility")})`,
        `- [Docs index](${md("/docs")})`,
        `- [Full text for agents](${SITE}/llms-full.txt)`,
      ].join("\n"),
    }),
  );

  files.set(
    "/compatibility",
    page({
      title: `Compatibility & scope (Amber ${version})`,
      summary: `What Amber ${version} supports: Stable contracts, Preview and Experimental surfaces, and deliberate non-goals, mirrored from docs/CURRENT_SCOPE.md.`,
      htmlRoute: "/compatibility",
      body: compatibilityBody(scope, conformance, scope.contractUrl),
    }),
  );

  const docsIndex = [];
  let lastGroup = null;
  for (const { group, doc } of navDocs) {
    if (group !== lastGroup) {
      docsIndex.push("", `## ${docGroups[group] || "Other"}`, "");
      lastGroup = group;
    }
    docsIndex.push(`- [${doc.title}](${md(`/docs/${doc.id}`)})${doc.subtitle ? `: ${doc.subtitle}` : ""}`);
  }
  files.set(
    "/docs",
    page({
      title: "Amber Docs",
      summary: `The Amber ${version} manual: install, CLI, TypeScript, testing, sandbox, Node and Web APIs.`,
      htmlRoute: "/docs",
      body: docsIndex.join("\n"),
    }),
  );

  for (const { doc } of navDocs) {
    files.set(
      `/docs/${doc.id}`,
      page({
        title: doc.title,
        summary: doc.subtitle,
        htmlRoute: `/docs/${doc.id}`,
        body: `${rewriteLinks(doc.body, knownRoutes)}\n\n---\n\nStatus labels and limits: [Compatibility](${md("/compatibility")}) · [Docs index](${md("/docs")}) · [Overview](${md("/")})`,
      }),
    );
  }

  for (const [route, content] of files) {
    const file = join(dist, markdownPathFor(route).slice(1));
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, content);
  }

  const docLink = ({ doc }) => `- [${doc.title}](${md(`/docs/${doc.id}`)})${doc.subtitle ? `: ${doc.subtitle}` : ""}`;
  const optionalGroups = new Set(["experimental"]);
  const llms = [
    "# Amber",
    "",
    `> ${summary}`,
    "",
    facts.replace(/^## Key facts\n\n/, ""),
    "## Overview",
    "",
    `- [Overview](${md("/")}): What Amber is and is not, install commands, license, and status summary`,
    `- [Compatibility & scope](${md("/compatibility")}): Every Stable contract with limits, plus Preview, Experimental, and non-goals`,
    `- [Docs index](${md("/docs")}): Table of contents for the manual`,
    "",
  ];
  const groups = [];
  for (const entry of navDocs) {
    let g = groups.find((x) => x.group === entry.group);
    if (!g) groups.push((g = { group: entry.group, items: [] }));
    g.items.push(entry);
  }
  for (const g of groups.filter((x) => !optionalGroups.has(x.group))) {
    llms.push(`## ${docGroups[g.group] || "Other"}`, "", ...g.items.map(docLink), "");
  }
  llms.push(
    "## Optional",
    "",
    ...groups.filter((x) => optionalGroups.has(x.group)).flatMap((g) => g.items.map(docLink)),
    `- [Full text of all pages](${SITE}/llms-full.txt): Every Markdown page above, concatenated`,
    `- [Release notes](${SITE}/blog): Blog posts (HTML). Older posts may quote historical benchmark numbers that are not current claims`,
    `- [Source code](${scope.REPO_URL}): GitHub repository`,
    `- [CURRENT_SCOPE.md](${scope.SCOPE_DOC_URL}): Authoritative capability boundary`,
    `- [Conformance scorecard](${scope.CONFORMANCE_URL}): ${conformance.pass}/${conformance.total} Node conformance fixtures`,
    "",
  );
  writeFileSync(join(dist, "llms.txt"), llms.join("\n"));

  const order = ["/", "/compatibility", ...navDocs.map(({ doc }) => `/docs/${doc.id}`)];
  const full = [
    "# Amber — full documentation for agents",
    "",
    `> ${summary}`,
    "",
    `Generated from ${SITE} at build time. Index: ${SITE}/llms.txt`,
    "",
    ...order.map((route) => `---\n\nURL: ${md(route)}\n\n${files.get(route)}`),
  ];
  writeFileSync(join(dist, "llms-full.txt"), full.join("\n"));

  console.log(`agent-docs: llms.txt, llms-full.txt and ${files.size} .md pages (blog posts: ${posts.length} skipped)`);
  return [...files.keys()];
}
