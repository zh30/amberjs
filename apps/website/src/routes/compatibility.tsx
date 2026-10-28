import { useMemo, useState } from "react";
import { ExternalLink, Search, X } from "lucide-react";
import { useLang } from "../lib/i18n";
import { landingCopy } from "../lib/landing-copy";
import { LevelBadge } from "../components/LevelBadge";
import {
  CONFORMANCE,
  CONFORMANCE_URL,
  EXPERIMENTAL,
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
    <ul className="mt-6 divide-y divide-[var(--line)] border border-[var(--line)]">
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

  const text = landingCopy[lang];

  return (
    <>
      <div className="sheet">
        <div className="lead">
          <h1>{text.compatTitle}</h1>
          <p className="sub">
            {text.compatBefore}
            <a href={SCOPE_DOC_URL}>docs/CURRENT_SCOPE.md</a>
            {text.compatAfter}
          </p>
        </div>
        <div className="install">
          <table className="level-table">
            <thead>
              <tr>
                <th>{text.levelHead}</th>
                <th>{text.meaningHead}</th>
              </tr>
            </thead>
            <tbody>
              {text.levels.map((row) => (
                <tr key={row.title}>
                  <td>{row.title}</td>
                  <td>{row.body}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="session">
            {text.scoreLine}
            {"\n"}
            <b>
              {CONFORMANCE.pass} / {CONFORMANCE.total}
            </b>
          </p>
        </div>
      </div>
    <div className="page">
        <p className="max-w-[70ch] text-base text-[var(--muted)]">
          {t.sourceNote}{" "}
          <a href={SCOPE_DOC_URL} className="font-semibold text-[var(--on-paper)] underline">
            CURRENT_SCOPE.md
          </a>
          . {copy.home.conformanceBody}{" "}
          <a href={CONFORMANCE_URL} className="font-semibold text-[var(--on-paper)] underline">
            {copy.home.conformanceLink}
          </a>
        </p>

        <nav className="mt-8 flex flex-wrap gap-2 text-sm">
          {toc.map((item) => (
            <a
              key={item.id}
              href={`#${item.id}`}
              className="inline-flex min-h-12 items-center border border-[var(--line)] px-3 text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
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
                  className={`inline-flex min-h-12 items-center border px-3 text-sm ${
                    area === a
                      ? "border-[var(--on-paper)] bg-[var(--honey)] text-[var(--honey-ink)]"
                      : "border-[var(--line)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  {a === "all" ? t.filterAll : t.areas[a]}
                  <span className="ml-1.5 font-mono text-xs">
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
                className="min-h-12 w-full border border-[var(--line)] bg-[var(--paper)] py-2 pl-9 pr-8 text-sm placeholder:text-[var(--text-muted)]"
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

          <div className="mt-4 overflow-hidden border border-[var(--line)]">
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
                        <span className="mt-0.5 block text-xs text-[var(--text-muted)]">
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
                          className="ml-2 inline-flex items-center gap-1 whitespace-nowrap font-mono text-xs text-[var(--text-primary)] underline underline-offset-2"
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
                className="border border-[var(--line-strong)] px-2 py-1 font-mono text-xs text-[var(--text-secondary)]"
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

        <section className="mt-20 border-t border-[var(--on-paper)] pt-10" id="graduation">
          <h2 className="scroll-mt-24 text-2xl font-bold tracking-tight">
            {t.graduationTitle}
          </h2>
          <ol className="mt-5 max-w-[70ch] space-y-4">
            {t.graduationRules.map((rule, i) => (
              <li key={rule} className="grid grid-cols-[2rem_1fr] gap-3 text-base leading-relaxed">
                <span className="font-mono text-sm text-[var(--muted)]">{i + 1}</span>
                <span className="text-[var(--text-secondary)]">{rule}</span>
              </li>
            ))}
          </ol>
        </section>
    </div>
    </>
  );
}
