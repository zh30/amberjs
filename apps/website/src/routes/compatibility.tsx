import { useMemo, useState } from "react";
import { ExternalLink, Search, X } from "lucide-react";
import { useLang } from "../lib/i18n";
import { LevelBadge } from "../components/LevelBadge";
import {
  CONFORMANCE,
  CONFORMANCE_URL,
  EXPERIMENTAL,
  GRADUATED_CONTRACTS,
  NON_GOALS,
  PREVIEW,
  SCOPE_DOC_URL,
  STABLE,
  UNCLASSIFIED,
  contractUrl,
  pick,
  type Area,
  type ScopeItem,
} from "../lib/scope";
import type { Level } from "../lib/locales/types";

const AREAS: Area[] = ["cli", "runtime", "node", "web"];

function ItemList({
  items,
  level,
  lang,
}: {
  items: readonly ScopeItem[];
  level: Level;
  lang: string;
}) {
  return (
    <ul className="mt-6 divide-y divide-[var(--line)] rounded-lg border border-[var(--line)]">
      {items.map((item) => (
        <li
          key={item.name}
          className="grid gap-2 p-4 sm:grid-cols-[minmax(0,18rem)_1fr] sm:gap-6"
        >
          <div className="flex items-start gap-2">
            <LevelBadge level={level} className="mt-0.5" />
            <code className="font-mono text-[13px] font-semibold">
              {item.name}
            </code>
          </div>
          <p className="text-sm leading-relaxed text-[var(--text-secondary)]">
            {pick(item.detail, lang)}
          </p>
        </li>
      ))}
    </ul>
  );
}

function Heading({
  id,
  level,
  title,
  subtitle,
}: {
  id: string;
  level?: Level;
  title: string;
  subtitle: string;
}) {
  return (
    <div id={id} className="scroll-mt-24">
      <h2 className="flex items-center gap-3 text-2xl font-bold tracking-tight">
        {title}
        {level ? <LevelBadge level={level} /> : null}
      </h2>
      <p className="mt-2 max-w-[70ch] text-[var(--text-secondary)]">
        {subtitle}
      </p>
    </div>
  );
}

export default function Compatibility() {
  const { copy, lang } = useLang();
  const t = copy.scope;
  const [area, setArea] = useState<Area | "all">("all");
  const [query, setQuery] = useState("");

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    return STABLE.filter((s) => area === "all" || s.area === area).filter(
      (s) =>
        !q ||
        [s.name, s.api, s.g ?? "", s.doc ?? "", s.limits.en, s.limits.zh]
          .join(" ")
          .toLowerCase()
          .includes(q),
    );
  }, [area, query]);

  const counts = useMemo(() => {
    const c: Record<string, number> = { all: STABLE.length };
    for (const a of AREAS) c[a] = STABLE.filter((s) => s.area === a).length;
    return c;
  }, []);

  const toc = [
    { id: "stable", label: t.stableTitle },
    { id: "preview", label: t.previewTitle },
    { id: "experimental", label: t.experimentalTitle },
    { id: "not-supported", label: t.nongoalTitle },
    { id: "graduation", label: t.graduationTitle },
  ];

  return (
    <div className="page">
      <div className="site-shell">
        <header className="max-w-[72ch]">
          <p className="font-mono text-xs text-[var(--text-muted)]">
            docs/CURRENT_SCOPE.md · G1–G{GRADUATED_CONTRACTS} ·{" "}
            {CONFORMANCE.pass}/{CONFORMANCE.total} conformance
          </p>
          <h1 className="mt-3 text-[clamp(2rem,4.5vw,3rem)] font-bold tracking-tight">
            {t.title}
          </h1>
          <p className="mt-4 text-lg leading-relaxed text-[var(--text-secondary)]">
            {t.subtitle}
          </p>
          <p className="mt-4 text-sm text-[var(--text-muted)]">
            {t.sourceNote}{" "}
            <a
              href={SCOPE_DOC_URL}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1 font-medium text-[var(--text-primary)] underline underline-offset-2"
            >
              CURRENT_SCOPE.md <ExternalLink className="h-3 w-3" />
            </a>
          </p>
        </header>

        <nav className="mt-8 flex flex-wrap gap-2 text-sm">
          {toc.map((item) => (
            <a
              key={item.id}
              href={`#${item.id}`}
              className="rounded-md border border-[var(--line)] px-3 py-1.5 text-[var(--text-secondary)] hover:border-[var(--text-muted)] hover:text-[var(--text-primary)]"
            >
              {item.label}
            </a>
          ))}
        </nav>

        <section className="mt-14">
          <Heading
            id="stable"
            level="stable"
            title={t.stableTitle}
            subtitle={t.stableSubtitle}
          />

          <div className="mt-6 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div
              role="tablist"
              aria-label={t.stableTitle}
              className="flex flex-wrap gap-1.5"
            >
              {(["all", ...AREAS] as const).map((a) => (
                <button
                  key={a}
                  type="button"
                  role="tab"
                  aria-selected={area === a}
                  onClick={() => setArea(a)}
                  className={`rounded-md border px-3 py-1.5 text-sm ${
                    area === a
                      ? "border-[var(--text-primary)] bg-[var(--fill)] text-[var(--fill-fg)]"
                      : "border-[var(--line)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  {a === "all" ? t.filterAll : t.areas[a]}
                  <span className="ml-1.5 font-mono text-[11px] opacity-60">
                    {counts[a]}
                  </span>
                </button>
              ))}
            </div>
            <label className="relative block lg:w-80">
              <span className="sr-only">{t.searchPlaceholder}</span>
              <Search
                className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--text-muted)]"
                aria-hidden
              />
              <input
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder={t.searchPlaceholder}
                className="w-full rounded-md border border-[var(--line)] bg-[var(--bg-page)] py-2 pl-9 pr-8 text-sm placeholder:text-[var(--text-muted)]"
              />
              {query ? (
                <button
                  type="button"
                  aria-label="Clear"
                  onClick={() => setQuery("")}
                  className="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                >
                  <X className="h-4 w-4" />
                </button>
              ) : null}
            </label>
          </div>

          <div className="mt-4 overflow-hidden rounded-lg border border-[var(--line)]">
            <div className="hidden grid-cols-[minmax(0,15rem)_minmax(0,6.5rem)_1fr] gap-6 border-b border-[var(--line)] bg-[var(--bg-raised)] px-4 py-2.5 text-xs font-medium text-[var(--text-muted)] md:grid">
              <span>{t.colSurface}</span>
              <span>{t.colId}</span>
              <span>{t.colLimits}</span>
            </div>
            {rows.length === 0 ? (
              <p className="p-6 text-sm text-[var(--text-muted)]">
                {t.noResults}
              </p>
            ) : (
              <ul className="divide-y divide-[var(--line)]">
                {rows.map((s) => (
                  <li
                    key={`${s.area}-${s.name}`}
                    className="grid gap-2 px-4 py-4 md:grid-cols-[minmax(0,15rem)_minmax(0,6.5rem)_1fr] md:gap-6"
                  >
                    <div className="min-w-0">
                      <div className="font-semibold">{s.name}</div>
                      <code className="mt-1 block break-words font-mono text-[12px] text-[var(--text-muted)]">
                        {s.api}
                      </code>
                    </div>
                    <div className="font-mono text-xs">
                      <span className="font-semibold">
                        {s.g ?? t.baseScope}
                      </span>
                      {s.qualifier ? (
                        <span className="mt-0.5 block text-[11px] text-[var(--text-muted)]">
                          {s.qualifier}
                        </span>
                      ) : null}
                    </div>
                    <div className="text-sm leading-relaxed text-[var(--text-secondary)]">
                      {pick(s.limits, lang)}
                      {s.doc ? (
                        <a
                          href={contractUrl(s.doc)}
                          target="_blank"
                          rel="noreferrer"
                          className="ml-2 inline-flex items-center gap-1 whitespace-nowrap font-mono text-[11px] text-[var(--text-primary)] underline underline-offset-2"
                        >
                          {t.contract} <ExternalLink className="h-3 w-3" />
                        </a>
                      ) : null}
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <p className="mt-3 text-xs text-[var(--text-muted)]">
            {t.qualifierNote}
          </p>
        </section>

        <section className="mt-20">
          <Heading
            id="preview"
            level="preview"
            title={t.previewTitle}
            subtitle={t.previewSubtitle}
          />
          <ItemList items={PREVIEW} level="preview" lang={lang} />
        </section>

        <section className="mt-20">
          <Heading
            id="experimental"
            level="experimental"
            title={t.experimentalTitle}
            subtitle={t.experimentalSubtitle}
          />
          <ItemList items={EXPERIMENTAL} level="experimental" lang={lang} />

          <h3 className="mt-10 text-lg font-semibold">{t.unclassifiedTitle}</h3>
          <p className="mt-2 max-w-[70ch] text-sm text-[var(--text-secondary)]">
            {t.unclassifiedSubtitle}
          </p>
          <ul className="mt-4 flex flex-wrap gap-2">
            {UNCLASSIFIED.map((name) => (
              <li
                key={name}
                className="rounded border border-dashed border-[var(--line-strong)] px-2 py-1 font-mono text-[12px] text-[var(--text-secondary)]"
              >
                {name}
              </li>
            ))}
          </ul>
        </section>

        <section className="mt-20">
          <Heading
            id="not-supported"
            level="nongoal"
            title={t.nongoalTitle}
            subtitle={t.nongoalSubtitle}
          />
          <ItemList items={NON_GOALS} level="nongoal" lang={lang} />
        </section>

        <section className="mt-20 grid gap-8 rounded-xl border border-[var(--line)] bg-[var(--bg-raised)] p-6 sm:p-10 lg:grid-cols-[1fr_1fr]">
          <div id="graduation" className="scroll-mt-24">
            <h2 className="text-2xl font-bold tracking-tight">
              {t.graduationTitle}
            </h2>
            <ol className="mt-5 space-y-3">
              {t.graduationRules.map((rule, i) => (
                <li key={rule} className="flex gap-3 text-sm leading-relaxed">
                  <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-[var(--honey)] font-mono text-xs font-semibold text-[var(--honey-ink)]">
                    {i + 1}
                  </span>
                  <span className="pt-0.5 text-[var(--text-secondary)]">
                    {rule}
                  </span>
                </li>
              ))}
            </ol>
          </div>
          <div className="flex flex-col justify-center rounded-lg bg-[var(--code-bg)] p-6 text-[var(--code-fg)]">
            <div className="font-mono text-5xl font-bold text-[var(--honey)]">
              {CONFORMANCE.pass}/{CONFORMANCE.total}
            </div>
            <p className="mt-3 text-sm leading-relaxed text-zinc-400">
              {copy.home.conformanceBody}
            </p>
            <a
              href={CONFORMANCE_URL}
              target="_blank"
              rel="noreferrer"
              className="mt-4 inline-flex items-center gap-1.5 text-sm font-medium text-white hover:underline"
            >
              {copy.home.conformanceLink}{" "}
              <ExternalLink className="h-3.5 w-3.5" />
            </a>
          </div>
        </section>
      </div>
    </div>
  );
}
