import { Link, Outlet, useLocation } from "react-router-dom";
import { AmberLogo } from "../components/Logo";
import { Mast } from "../components/Mast";
import "../global.css";
import { LangProvider, useLang } from "../lib/i18n";
import { InstallProvider } from "../lib/install-choice";
import { landingCopy } from "../lib/landing-copy";
import { RouteScroll } from "../lib/route-scroll";
import { AMBER_VERSION } from "../lib/version";
import { ThemeProvider } from "../lib/theme";
import { CONFORMANCE_URL, REPO_URL, SCOPE_DOC_URL } from "../lib/scope";

const AGENT_DOCS = [
  { key: "llms", href: "/llms.txt" },
  { key: "llmsFull", href: "/llms-full.txt" },
  { key: "overview", href: "/index.md" },
  { key: "compatibility", href: "/compatibility.md" },
  { key: "install", href: "/docs/installation.md" },
  { key: "cli", href: "/docs/cli-usage.md" },
] as const;

const ASK_AI = [
  { name: "ChatGPT", base: "https://chatgpt.com/?q=" },
  { name: "Claude", base: "https://claude.ai/new?q=" },
  { name: "Perplexity", base: "https://www.perplexity.ai/search?q=" },
  { name: "Google AI Mode", base: "https://www.google.com/search?udm=50&q=" },
] as const;

function RootLayoutInner() {
  const { copy, lang } = useLang();
  const location = useLocation();
  const isPlay = location.pathname === "/play";
  const isHome = location.pathname === "/";

  const footerColumns = [
    {
      title: copy.footer.product,
      links: [
        { label: copy.footer.docs, to: "/docs" },
        { label: copy.footer.scope, to: "/compatibility" },
        { label: copy.footer.play, to: "/play" },
        { label: copy.footer.blog, to: "/blog" },
      ],
    },
    {
      title: copy.footer.resources,
      links: [
        { label: copy.footer.conformance, href: CONFORMANCE_URL },
        { label: "CURRENT_SCOPE.md", href: SCOPE_DOC_URL },
        { label: copy.footer.releases, href: `${REPO_URL}/releases` },
      ],
    },
    {
      title: copy.footer.community,
      links: [
        { label: copy.footer.githubRepo, href: REPO_URL },
        { label: copy.footer.issues, href: `${REPO_URL}/issues` },
        { label: copy.footer.license, href: `${REPO_URL}/blob/main/LICENSE` },
      ],
    },
  ];

  return (
    <div
      className={`flex flex-col bg-[var(--paper)] text-[var(--on-paper)] ${
        isPlay ? "h-dvh overflow-hidden" : "min-h-screen"
      }`}
    >
      <a className="skip" href="#main">
        {landingCopy[lang].skip}
      </a>
      <RouteScroll />
      {isHome ? null : (
        <div className="site-mast">
          <Mast />
        </div>
      )}

      <main id="main" className={isPlay ? "flex min-h-0 grow flex-col" : "grow"}>
        <Outlet />
      </main>

      {isPlay ? null : (
        <footer className="site-footer">
          <div className="grid gap-10 text-base md:grid-cols-[1.4fr_repeat(3,1fr)]">
            <div className="max-w-xs">
              <div className="flex items-center gap-2">
                <AmberLogo className="h-6 w-6" />
                <span className="font-semibold">Amber</span>
                <span className="font-mono text-sm text-[var(--muted)]">
                  {AMBER_VERSION}
                </span>
              </div>
              <p className="mt-3 text-[var(--muted)]">{copy.footer.tagline}</p>
            </div>
            {footerColumns.map((column) => (
              <div key={column.title}>
                <div className="font-semibold">{column.title}</div>
                <ul className="mt-3 space-y-2">
                  {column.links.map((link) => (
                    <li key={link.label}>
                      {"to" in link && link.to ? (
                        <Link
                          to={link.to}
                          className="font-semibold text-[var(--on-paper)] hover:underline"
                        >
                          {link.label}
                        </Link>
                      ) : (
                        <a
                          href={"href" in link ? link.href : undefined}
                          target="_blank"
                          rel="noreferrer"
                          className="font-semibold text-[var(--on-paper)] hover:underline"
                        >
                          {link.label}
                        </a>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          <section
            aria-labelledby="footer-agents"
            className="mt-10 grid gap-6 border-t border-[var(--on-paper)] pt-8 md:grid-cols-[1.4fr_3fr]"
          >
            <div className="max-w-xs">
              <h2 id="footer-agents" className="font-semibold">
                {copy.footer.agents.title}
              </h2>
              <p className="mt-3 text-[var(--muted)]">{copy.footer.agents.intro}</p>
            </div>
            <div className="grid gap-5">
              <ul className="flex flex-wrap gap-x-5 gap-y-2 font-mono text-sm">
                {AGENT_DOCS.map((doc) => (
                  <li key={doc.href}>
                    <a
                      href={doc.href}
                      className="font-semibold text-[var(--on-paper)] hover:underline"
                    >
                      {copy.footer.agents[doc.key]}
                    </a>
                  </li>
                ))}
              </ul>
              <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
                <span className="text-[var(--muted)]">{copy.footer.agents.ask}</span>
                {ASK_AI.map((ai) => (
                  <a
                    key={ai.name}
                    href={`${ai.base}${encodeURIComponent(copy.footer.agents.prompt)}`}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex min-h-12 items-center border border-[var(--on-paper)] px-3 text-sm font-semibold"
                  >
                    {ai.name}
                  </a>
                ))}
              </div>
            </div>
          </section>
          <div className="mt-8 flex flex-col gap-2 border-t border-[var(--on-paper)] pt-5 text-sm text-[var(--muted)] sm:flex-row sm:justify-between">
            <span>© {new Date().getFullYear()} Amber contributors · MIT</span>
            <span className="font-mono">amberjs.com</span>
          </div>
        </footer>
      )}
    </div>
  );
}

export default function RootLayout() {
  return (
    <ThemeProvider>
      <LangProvider>
        <InstallProvider>
          <RootLayoutInner />
        </InstallProvider>
      </LangProvider>
    </ThemeProvider>
  );
}
