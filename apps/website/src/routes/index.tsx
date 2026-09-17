import { useState } from "react";
import { Check, Copy, Terminal } from "lucide-react";
import { Link } from "react-router-dom";
import { useLang } from "../lib/i18n";
import { BEEJS_VERSION } from "../lib/version";

const INSTALL = "curl -fsSL https://amberjs.com/install.sh | sh";

export default function HomeComponent() {
  const { copy } = useLang();
  const home = copy.home;
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(INSTALL);
    setCopied(true);
    setTimeout(() => setCopied(false), 1600);
  };

  return (
    <div className="pb-24">
      <section className="bg-[var(--honey)] text-[var(--honey-ink)]">
        <div className="site-shell grid items-end gap-10 py-16 sm:py-20 lg:grid-cols-[1.1fr_0.9fr] lg:py-24">
          <div>
            <p className="font-mono text-sm font-medium">{BEEJS_VERSION}</p>
            <h1 className="mt-4 max-w-[16ch] text-[clamp(2.5rem,6vw,4.25rem)] font-bold leading-[1.05]">
              {home.heroTitlePrefix}
              {home.heroTitleAccent}
              {home.heroTitleSuffix}
            </h1>
            <p className="mt-6 max-w-[42ch] text-lg leading-relaxed sm:text-xl">
              {home.heroSubtitle}
            </p>
            <div className="mt-8 flex flex-wrap gap-3 text-sm font-medium">
              <Link
                to="/docs"
                className="bg-[var(--honey-ink)] px-5 py-2.5 font-medium text-[var(--honey)]"
              >
                {home.ctaPrimary}
              </Link>
              <Link
                to="/docs/installation"
                className="border border-[var(--honey-ink)] px-5 py-2.5 font-medium"
              >
                {home.ctaButton}
              </Link>
            </div>
          </div>

          <div className="min-w-0 bg-[var(--code-bg)] text-[var(--code-fg)]">
            <div className="flex items-center justify-between gap-3 border-b border-white/10 px-4 py-2.5 font-mono text-[12px]">
              <span className="inline-flex items-center gap-2">
                <Terminal className="h-3.5 w-3.5 opacity-70" aria-hidden />
                install
              </span>
              <button
                type="button"
                onClick={handleCopy}
                className="hover:text-white"
              >
                {copied ? (
                  <span className="inline-flex items-center gap-1">
                    <Check className="h-3.5 w-3.5" /> {home.copiedBtn}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1">
                    <Copy className="h-3.5 w-3.5" /> {home.copyBtn}
                  </span>
                )}
              </button>
            </div>
            <div className="overflow-x-auto px-4 py-3 font-mono text-[13px]">
              <code className="whitespace-nowrap">{INSTALL}</code>
            </div>
            <pre className="overflow-x-auto border-t border-white/10 px-4 py-4 font-mono text-[13px] leading-relaxed">
              {`// hello.ts
const runtime = "Amber";
console.log(\`hello from \${runtime}\`);

$ amber run hello.ts
hello from Amber`}
            </pre>
          </div>
        </div>
      </section>

      <section className="site-shell py-16 sm:py-20">
        <h2 className="text-3xl font-bold tracking-tight">
          {home.featuresTitle}
        </h2>
        <p className="mt-3 max-w-[52ch] text-lg text-[var(--text-secondary)]">
          {home.featuresSubtitle}
        </p>
        <dl className="mt-12 grid gap-x-10 gap-y-8 sm:grid-cols-2">
          {home.features.map((item) => (
            <div
              key={item.title}
              className="border-t border-[var(--line)] pt-4"
            >
              <dt className="text-lg font-semibold">{item.title}</dt>
              <dd className="mt-2 text-[var(--text-secondary)]">{item.desc}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section className="border-y border-[var(--line)] bg-[var(--code-bg)] py-16 text-[var(--code-fg)] sm:py-20">
        <div className="site-shell">
          <h2 className="text-3xl font-bold tracking-tight">
            {home.telemetryTitle}
          </h2>
          <p className="mt-3 max-w-[62ch] text-[15px] text-zinc-400">
            {home.telemetrySubtitle}
          </p>
          <div className="mt-10 overflow-x-auto">
            <table className="w-full min-w-[40rem] text-left text-sm">
              <thead>
                <tr className="border-b border-white/15">
                  <th className="py-3 pr-4 font-medium text-zinc-400">
                    {home.benchmarksFilterAll}
                  </th>
                  <th className="bg-[var(--honey)] px-3 py-3 font-mono font-bold text-[var(--honey-ink)]">
                    Beejs
                  </th>
                  <th className="py-3 pr-4 pl-4 font-mono font-medium text-zinc-400">
                    Node
                  </th>
                  <th className="py-3 font-mono font-medium text-zinc-400">
                    Bun
                  </th>
                </tr>
              </thead>
              <tbody>
                {home.benchmarks.map((row) => (
                  <tr key={row.id} className="border-b border-white/10">
                    <td className="py-3.5 pr-4">
                      <div className="font-medium">{row.title}</div>
                      <div className="text-zinc-500">{row.desc}</div>
                    </td>
                    <td className="bg-[var(--honey)] px-3 py-3.5 font-mono font-semibold whitespace-nowrap text-[var(--honey-ink)]">
                      {row.beeValue}
                    </td>
                    <td className="py-3.5 pr-4 pl-4 font-mono whitespace-nowrap text-zinc-300">
                      {row.nodeValue}
                    </td>
                    <td className="py-3.5 font-mono whitespace-nowrap text-zinc-300">
                      {row.bunValue}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="mt-5 max-w-[70ch] text-sm text-zinc-500">
            {home.benchmarksNote}
          </p>
        </div>
      </section>

      <section className="site-shell py-16 sm:py-20">
        <h2 className="text-3xl font-bold tracking-tight">{home.ctaTitle}</h2>
        <p className="mt-3 max-w-[50ch] text-lg text-[var(--text-secondary)]">
          {home.ctaSubtitle}
        </p>
        <div className="mt-8 flex flex-wrap gap-3 text-sm font-medium">
          <Link to="/docs/installation" className="btn-honey">
            {home.ctaButton}
          </Link>
          <Link to={home.latestArticle.link} className="btn-line">
            {home.ctaNotesButton}
          </Link>
        </div>
      </section>
    </div>
  );
}
