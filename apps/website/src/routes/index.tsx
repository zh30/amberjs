import { useState } from "react";
import { Link } from "react-router-dom";
import { ArrowRight, Check, Copy, ExternalLink, X } from "lucide-react";
import { useLang } from "../lib/i18n";
import { AMBER_VERSION } from "../lib/version";
import { CodeWindow } from "../components/CodeWindow";
import { LevelBadge } from "../components/LevelBadge";
import {
  CONFORMANCE,
  CONFORMANCE_URL,
  GRADUATED_CONTRACTS,
  NON_GOALS,
  STABLE,
  pick,
  type Area,
} from "../lib/scope";
import type {
  CommandKey,
  Level,
  ShowcaseId,
} from "../lib/locales/types";

const INSTALL = {
  unix: "curl -fsSL https://get.amberjs.com/install.sh | sh",
  windows: "irm https://get.amberjs.com/install.ps1 | iex",
  source:
    "git clone https://github.com/zh30/amberjs && cd amberjs && cargo build --release",
} as const;
type InstallKey = keyof typeof INSTALL;

const SHOWCASE: Record<ShowcaseId, { file: string; code: string }> = {
  typescript: {
    file: "hello.ts",
    code: `// hello.ts
interface Greeting {
  name: string;
}

const greet = ({ name }: Greeting): string => \`hello from \${name}\`;
console.log(greet({ name: "Amber" }));

$ amber run hello.ts
hello from Amber`,
  },
  test: {
    file: "math.test.js",
    code: `// math.test.js
describe("math", () => {
  test("adds numbers", () => {
    expect(2 + 3).toBe(5);
  });

  test("maps arrays", () => {
    expect([1, 2, 3].map((n) => n * 2)).toEqual([2, 4, 6]);
  });
});

$ amber test
$ amber test --watch`,
  },
  serve: {
    file: "server.js",
    code: `// server.js
module.exports = {
  fetch(request) {
    const { pathname } = new URL(request.url);
    return new Response(\`hello \${pathname}\`);
  },
};

$ amber serve --port 3000 server.js
$ curl http://localhost:3000/amber
hello /amber`,
  },
  sandbox: {
    file: "tool.js",
    code: `// tool.js
const fs = require("fs");
console.log(fs.readFileSync("./data/input.txt", "utf8"));
fs.readFileSync("/etc/passwd"); // throws: permission denied

$ amber run --sandbox \\
    --allow-read ./data/input.txt \\
    --audit-log audit.jsonl \\
    tool.js`,
  },
  tools: {
    file: "tools.ts",
    code: `// tools.ts
/** Add two numbers */
export function add(a: number, b: number) {
  return a + b;
}

$ amber run --export-tools tools.ts
{"tools":[{"name":"add","description":"Add two numbers","inputSchema":{"type":"object"}}]}
$ amber mcp tools.ts`,
  },
};

const COMMANDS: { key: CommandKey; cmd: string; doc: string }[] = [
  { key: "run", cmd: "amber run", doc: "quick-start" },
  { key: "test", cmd: "amber test", doc: "testing" },
  { key: "serve", cmd: "amber serve", doc: "server-mode" },
  { key: "bundle", cmd: "amber bundle", doc: "bundling-compilation" },
  { key: "compile", cmd: "amber compile", doc: "bundling-compilation" },
  { key: "install", cmd: "amber install", doc: "package-manager" },
  { key: "mcp", cmd: "amber mcp", doc: "mcp-protocol" },
  { key: "repl", cmd: "amber repl", doc: "cli-usage" },
];

const LEVELS: Level[] = ["stable", "preview", "experimental", "nongoal"];

function chipsFor(area: Area): string[] {
  return STABLE.filter((s) => s.area === area).map((s) => s.name);
}

function InstallCard() {
  const { copy } = useLang();
  const home = copy.home;
  const [tab, setTab] = useState<InstallKey>("unix");
  const [copied, setCopied] = useState(false);

  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(INSTALL[tab]);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch {
      /* clipboard unavailable */
    }
  };

  return (
    <div className="min-w-0 overflow-hidden rounded-lg border border-black/15 bg-[var(--code-bg)] text-[var(--code-fg)] shadow-[0_24px_48px_-24px_rgba(20,18,11,0.55)]">
      <div
        role="tablist"
        aria-label="Install"
        className="flex gap-1 overflow-x-auto border-b border-white/10 px-2 pt-2"
      >
        {(Object.keys(INSTALL) as InstallKey[]).map((key) => (
          <button
            key={key}
            role="tab"
            type="button"
            aria-selected={tab === key}
            onClick={() => setTab(key)}
            className={`shrink-0 border-b-2 px-3 py-2 font-mono text-[12px] ${
              tab === key
                ? "border-[var(--honey)] text-white"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            {home.installTabs[key]}
          </button>
        ))}
      </div>
      <div className="flex items-start gap-3 px-4 py-4">
        <pre className="m-0 min-w-0 flex-1 overflow-x-auto font-mono text-[13px] leading-relaxed">
          <code>
            <span className="select-none text-[var(--honey)]">$ </span>
            {INSTALL[tab]}
          </code>
        </pre>
        <button
          type="button"
          onClick={onCopy}
          aria-label={home.copyBtn}
          className="mt-0.5 shrink-0 text-zinc-400 hover:text-white"
        >
          {copied ? (
            <Check className="h-4 w-4 text-emerald-400" />
          ) : (
            <Copy className="h-4 w-4" />
          )}
        </button>
      </div>
      <pre className="m-0 overflow-x-auto border-t border-white/10 px-4 py-4 font-mono text-[13px] leading-[1.7]">
        <code>
          <div>
            <span className="text-[var(--honey)]">$ </span>
            <span className="text-white">amber --version</span>
          </div>
          <div className="text-zinc-400">amber {AMBER_VERSION.slice(1)}</div>
          <div>
            <span className="text-[var(--honey)]">$ </span>
            <span className="text-white">amber eval "1 + 1"</span>
          </div>
          <div className="text-zinc-400">2</div>
        </code>
      </pre>
    </div>
  );
}

function SectionHeader({
  title,
  subtitle,
}: {
  title: string;
  subtitle: string;
}) {
  return (
    <div className="max-w-[60ch]">
      <h2 className="text-[clamp(1.75rem,3.2vw,2.35rem)] font-bold tracking-tight">
        {title}
      </h2>
      <p className="mt-3 text-lg leading-relaxed text-[var(--text-secondary)]">
        {subtitle}
      </p>
    </div>
  );
}

export default function HomeComponent() {
  const { copy, lang } = useLang();
  const home = copy.home;
  const [active, setActive] = useState<ShowcaseId>("typescript");
  const showcaseIds = Object.keys(SHOWCASE) as ShowcaseId[];

  const stats = [
    { value: String(GRADUATED_CONTRACTS), label: home.stats.contracts },
    {
      value: `${CONFORMANCE.pass}/${CONFORMANCE.total}`,
      label: home.stats.conformance,
    },
    { value: "1", label: home.stats.binary },
    { value: AMBER_VERSION, label: home.stats.version },
  ];

  const compatColumns: { title: string; area: Area }[] = [
    { title: home.compatNode, area: "node" },
    { title: home.compatWeb, area: "web" },
    { title: home.compatRuntime, area: "runtime" },
  ];

  return (
    <div>
      {/* Hero */}
      <section className="relative overflow-hidden bg-[var(--honey)] text-[var(--honey-ink)]">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 opacity-[0.08] [background-image:radial-gradient(var(--honey-ink)_1px,transparent_1px)] [background-size:22px_22px]"
        />
        <div className="site-shell relative grid items-center gap-12 py-16 sm:py-20 lg:grid-cols-[1.05fr_0.95fr] lg:py-24">
          <div>
            <p className="inline-flex items-center gap-2 rounded-full border border-[var(--honey-ink)]/25 px-3 py-1 font-mono text-xs font-medium">
              <span className="h-1.5 w-1.5 rounded-full bg-[var(--honey-ink)]" />
              {AMBER_VERSION} · {home.eyebrow}
            </p>
            <h1 className="mt-6 max-w-[18ch] text-[clamp(2.5rem,6vw,4.25rem)] font-bold leading-[1.04] tracking-[-0.03em]">
              {home.titleLead}
              <span className="underline decoration-[var(--honey-ink)]/30 decoration-[0.12em] underline-offset-[0.14em]">
                {home.titleAccent}
              </span>
            </h1>
            <p className="mt-6 max-w-[46ch] text-lg leading-relaxed sm:text-xl">
              {home.subtitle}
            </p>
            <div className="mt-9 flex flex-wrap gap-3 text-sm font-medium">
              <Link
                to="/docs/quick-start"
                className="inline-flex items-center gap-1.5 rounded-md bg-[var(--honey-ink)] px-5 py-2.5 text-[var(--honey)] hover:opacity-90"
              >
                {home.ctaPrimary} <ArrowRight className="h-4 w-4" />
              </Link>
              <Link
                to="/compatibility"
                className="inline-flex items-center gap-1.5 rounded-md border border-[var(--honey-ink)] px-5 py-2.5 hover:bg-[var(--honey-ink)]/5"
              >
                {home.ctaSecondary}
              </Link>
            </div>
          </div>
          <InstallCard />
        </div>
      </section>

      {/* Stats */}
      <section className="border-b border-[var(--line)]">
        <dl className="site-shell grid grid-cols-2 lg:grid-cols-4">
          {stats.map((s, i) => (
            <div
              key={s.label}
              className={`border-[var(--line)] py-7 ${
                i % 2 === 1 ? "pl-5 sm:pl-8" : "pr-5"
              } ${i > 0 ? "lg:border-l lg:pl-8" : ""} ${
                i % 2 === 1 ? "border-l" : ""
              } ${i < 2 ? "border-b lg:border-b-0" : ""}`}
            >
              <dt className="sr-only">{s.label}</dt>
              <dd>
                <div className="font-mono text-2xl font-semibold tracking-tight sm:text-3xl">
                  {s.value}
                </div>
                <div className="mt-1 text-sm text-[var(--text-muted)]">
                  {s.label}
                </div>
              </dd>
            </div>
          ))}
        </dl>
      </section>

      {/* Showcase */}
      <section className="site-shell py-20 sm:py-24">
        <SectionHeader
          title={home.showcaseTitle}
          subtitle={home.showcaseSubtitle}
        />
        <div className="mt-10 grid gap-6 lg:grid-cols-[260px_1fr]">
          <div
            role="tablist"
            aria-label={home.showcaseTitle}
            className="flex gap-2 overflow-x-auto pb-1 lg:flex-col lg:overflow-visible lg:pb-0"
          >
            {showcaseIds.map((id) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={active === id}
                onClick={() => setActive(id)}
                className={`shrink-0 rounded-md border px-4 py-3 text-left text-sm lg:w-full ${
                  active === id
                    ? "border-[var(--text-primary)] bg-[var(--bg-raised)] font-semibold"
                    : "border-[var(--line)] text-[var(--text-secondary)] hover:border-[var(--text-muted)] hover:text-[var(--text-primary)]"
                }`}
              >
                <span className="block">{home.showcase[id].label}</span>
                <span className="mt-1 hidden text-xs font-normal leading-snug text-[var(--text-muted)] lg:block">
                  {home.showcase[id].caption}
                </span>
              </button>
            ))}
          </div>
          <div className="min-w-0">
            <CodeWindow
              title={SHOWCASE[active].file}
              code={SHOWCASE[active].code}
              copyLabel={home.copyBtn}
              copiedLabel={home.copiedBtn}
            />
            <p className="mt-4 flex items-start gap-2 text-sm text-[var(--text-secondary)] lg:hidden">
              {home.showcase[active].caption}
            </p>
          </div>
        </div>
      </section>

      {/* Commands */}
      <section className="border-y border-[var(--line)] bg-[var(--bg-raised)] py-20 sm:py-24">
        <div className="site-shell">
          <SectionHeader
            title={home.commandsTitle}
            subtitle={home.commandsSubtitle}
          />
          <div className="mt-10 grid gap-px overflow-hidden rounded-lg border border-[var(--line)] bg-[var(--line)] sm:grid-cols-2 lg:grid-cols-4">
            {COMMANDS.map((c) => (
              <Link
                key={c.key}
                to={`/docs/${c.doc}`}
                className="group flex flex-col gap-2 bg-[var(--bg-page)] p-5 hover:bg-[var(--bg-raised)]"
              >
                <div className="flex items-center justify-between gap-2">
                  <code className="font-mono text-[14px] font-semibold">
                    {c.cmd}
                  </code>
                  <LevelBadge level="stable" />
                </div>
                <p className="text-sm leading-snug text-[var(--text-secondary)]">
                  {home.commands[c.key]}
                </p>
              </Link>
            ))}
          </div>
          <Link
            to="/docs/cli-usage"
            className="mt-6 inline-flex items-center gap-1.5 text-sm font-medium hover:underline"
          >
            {home.commandsMore} <ArrowRight className="h-4 w-4" />
          </Link>
        </div>
      </section>

      {/* Compatibility */}
      <section className="site-shell py-20 sm:py-24">
        <SectionHeader title={home.compatTitle} subtitle={home.compatSubtitle} />
        <div className="mt-10 grid gap-10 lg:grid-cols-3">
          {compatColumns.map((col) => (
            <div key={col.area}>
              <h3 className="flex items-center gap-2 border-b border-[var(--line)] pb-3 text-base font-semibold">
                {col.title}
                <span className="font-mono text-xs font-normal text-[var(--text-muted)]">
                  {chipsFor(col.area).length}
                </span>
              </h3>
              <ul className="mt-4 flex flex-wrap gap-2">
                {chipsFor(col.area).map((name) => (
                  <li
                    key={name}
                    className="rounded border border-[var(--line)] bg-[var(--bg-raised)] px-2 py-1 font-mono text-[12px]"
                  >
                    {name}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
        <Link
          to="/compatibility"
          className="btn-ink mt-10 rounded-md"
        >
          {home.compatLink} <ArrowRight className="h-4 w-4" />
        </Link>
      </section>

      {/* Conformance */}
      <section className="bg-[var(--code-bg)] py-20 text-[var(--code-fg)] sm:py-24">
        <div className="site-shell grid items-center gap-12 lg:grid-cols-[0.9fr_1.1fr]">
          <div>
            <div className="font-mono text-[clamp(3.5rem,9vw,6rem)] font-bold leading-none tracking-tight text-[var(--honey)]">
              {CONFORMANCE.pass}/{CONFORMANCE.total}
            </div>
            <h2 className="mt-4 text-2xl font-bold tracking-tight text-white sm:text-3xl">
              {home.conformanceTitle}
            </h2>
            <p className="mt-4 max-w-[56ch] leading-relaxed text-zinc-400">
              {home.conformanceBody}
            </p>
            <a
              href={CONFORMANCE_URL}
              target="_blank"
              rel="noreferrer"
              className="mt-6 inline-flex items-center gap-1.5 text-sm font-medium text-white hover:underline"
            >
              {home.conformanceLink} <ExternalLink className="h-3.5 w-3.5" />
            </a>
          </div>
          <div>
            <p className="mb-3 font-mono text-xs text-zinc-500">
              {home.conformanceReproduce}
            </p>
            <pre className="m-0 overflow-x-auto rounded-lg border border-white/10 bg-black/30 p-5 font-mono text-[13px] leading-[1.7]">
              <code>
                <div>
                  <span className="text-[var(--honey)]">$ </span>
                  <span className="text-white">cargo build --release</span>
                </div>
                <div>
                  <span className="text-[var(--honey)]">$ </span>
                  <span className="text-white">
                    AMBER_BIN=./target/release/amber \
                  </span>
                </div>
                <div className="text-white">
                  {"    "}./tests/conformance/run_conformance.sh
                </div>
                <div className="text-zinc-500">…</div>
                <div className="text-zinc-400">
                  Pass rate: {CONFORMANCE.pass}/{CONFORMANCE.total} (100%)
                </div>
                <div className="text-emerald-400">
                  CI gate: PASS (&gt;= {CONFORMANCE.gate} PASS, 0 FAIL, 0 SKIP)
                </div>
              </code>
            </pre>
          </div>
        </div>
      </section>

      {/* Maturity */}
      <section className="site-shell py-20 sm:py-24">
        <SectionHeader
          title={home.maturityTitle}
          subtitle={home.maturitySubtitle}
        />
        <div className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {LEVELS.map((level) => (
            <div
              key={level}
              className="rounded-lg border border-[var(--line)] p-5"
            >
              <LevelBadge level={level} />
              <p className="mt-4 text-sm leading-relaxed text-[var(--text-secondary)]">
                {copy.levels[level].desc}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Not supported */}
      <section className="border-t border-[var(--line)] bg-[var(--bg-raised)] py-20 sm:py-24">
        <div className="site-shell">
          <SectionHeader title={home.notTitle} subtitle={home.notSubtitle} />
          <ul className="mt-10 grid gap-x-10 gap-y-6 sm:grid-cols-2 lg:grid-cols-3">
            {NON_GOALS.map((item) => (
              <li key={item.name} className="flex gap-3">
                <X
                  className="mt-0.5 h-4 w-4 shrink-0 text-rose-500"
                  aria-hidden
                />
                <div>
                  <div className="font-mono text-[13px] font-semibold">
                    {item.name}
                  </div>
                  <p className="mt-1 text-sm leading-relaxed text-[var(--text-secondary)]">
                    {pick(item.detail, lang)}
                  </p>
                </div>
              </li>
            ))}
          </ul>
          <div className="mt-14 max-w-[70ch] rounded-lg border border-dashed border-[var(--line-strong)] p-5">
            <h3 className="text-sm font-semibold">{home.perfTitle}</h3>
            <p className="mt-2 text-sm leading-relaxed text-[var(--text-secondary)]">
              {home.perfBody}
            </p>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="site-shell py-20 sm:py-24">
        <div className="grid items-center gap-8 rounded-xl bg-[var(--honey)] p-8 text-[var(--honey-ink)] sm:p-12 lg:grid-cols-[1fr_auto]">
          <div>
            <h2 className="text-[clamp(1.75rem,3.2vw,2.5rem)] font-bold tracking-tight">
              {home.ctaTitle}
            </h2>
            <p className="mt-3 max-w-[52ch] text-lg">{home.ctaSubtitle}</p>
          </div>
          <div className="flex flex-wrap gap-3 text-sm font-medium">
            <Link
              to="/docs/installation"
              className="inline-flex items-center gap-1.5 rounded-md bg-[var(--honey-ink)] px-5 py-2.5 text-[var(--honey)]"
            >
              {home.ctaButton} <ArrowRight className="h-4 w-4" />
            </Link>
            <Link
              to="/blog"
              className="inline-flex items-center rounded-md border border-[var(--honey-ink)] px-5 py-2.5"
            >
              {home.ctaNotesButton}
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
