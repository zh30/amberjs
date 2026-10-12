import { useState, useRef, useEffect } from "react";
import { Link, Outlet, useLocation } from "react-router-dom";
import { AmberLogo } from "../components/Logo";
import "../global.css";
import { LangProvider, useLang } from "../lib/i18n";
import { RouteScroll } from "../lib/route-scroll";
import { AMBER_VERSION } from "../lib/version";
import { ThemeProvider, useTheme } from "../lib/theme";
import {
  Check,
  ChevronDown,
  Github,
  Globe,
  Menu,
  Monitor,
  Moon,
  Sun,
  X,
} from "lucide-react";
import { CONFORMANCE_URL, REPO_URL, SCOPE_DOC_URL } from "../lib/scope";

function LanguageSelector() {
  const { lang, setLang, copy, languages } = useLang();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const currentOption = languages.find((l) => l.code === lang) || languages[0];

  useEffect(() => {
    function handleClickOutside(event: MouseEvent | TouchEvent) {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    }

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("touchstart", handleClickOutside);
      document.addEventListener("keydown", handleKeyDown);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("touchstart", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        className="inline-flex items-center gap-1.5 px-2 py-1 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
        aria-label={copy.toggle.label}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
      >
        <Globe className="w-3.5 h-3.5 text-zinc-500 dark:text-zinc-400 shrink-0" />
        <span className="text-[11px] font-medium hidden sm:inline">
          {currentOption.nativeLabel}
        </span>
        <span className="text-[11px] font-medium sm:hidden">
          {currentOption.code.toUpperCase()}
        </span>
        <ChevronDown className={`h-3 w-3 ${isOpen ? "rotate-180" : ""}`} />
      </button>

      {isOpen && (
        <div
          role="listbox"
          aria-label={copy.toggle.label}
          className="absolute right-0 top-full z-50 mt-1 w-48 border border-[var(--line)] bg-[var(--bg-page)] p-1"
        >
          <div className="border-b border-[var(--line)] px-2.5 py-1.5 text-[10px] text-[var(--text-muted)]">
            {copy.toggle.label}
          </div>
          {languages.map((option) => {
            const isSelected = option.code === lang;
            return (
              <button
                key={option.code}
                type="button"
                role="option"
                aria-selected={isSelected}
                onClick={() => {
                  setLang(option.code);
                  setIsOpen(false);
                }}
                className={`flex w-full items-center justify-between px-2.5 py-2 text-xs ${
                  isSelected
                    ? "font-semibold text-[var(--text-primary)]"
                    : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm leading-none">{option.flag}</span>
                  <span className="font-medium">{option.nativeLabel}</span>
                  <span className="text-[10px] font-mono text-zinc-400 dark:text-zinc-500">
                    ({option.label})
                  </span>
                </div>
                {isSelected && <Check className="ml-2 h-3.5 w-3.5 shrink-0" />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

const NAV_LINKS = [
  { to: "/docs", key: "docs" },
  { to: "/compatibility", key: "scope" },
  { to: "/blog", key: "blog" },
  { to: "/play", key: "play" },
] as const;

const AGENT_DOCS = [
  { key: "llms", href: "/llms.txt" },
  { key: "llmsFull", href: "/llms-full.txt" },
  { key: "overview", href: "/index.md" },
  { key: "compatibility", href: "/compatibility.md" },
  { key: "install", href: "/docs/installation.md" },
  { key: "cli", href: "/docs/cli-usage.md" },
] as const;

// Only chat apps whose URL query actually prefills a prompt; gemini.google.com/app
// ignores ?q=, so Gemini is reached through Google AI Mode (udm=50) instead.
const ASK_AI = [
  { name: "ChatGPT", base: "https://chatgpt.com/?q=" },
  { name: "Claude", base: "https://claude.ai/new?q=" },
  { name: "Perplexity", base: "https://www.perplexity.ai/search?q=" },
  { name: "Google AI Mode", base: "https://www.google.com/search?udm=50&q=" },
] as const;

function isActive(pathname: string, to: string) {
  return pathname === to || pathname.startsWith(`${to}/`);
}

function RootLayoutInner() {
  const { copy } = useLang();
  const { theme, toggleNext } = useTheme();
  const location = useLocation();
  const isPlay = location.pathname === "/play";
  const [menuPath, setMenuPath] = useState<string | null>(null);
  const menuOpen = menuPath === location.pathname;

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
      className={`flex flex-col bg-[var(--bg-page)] text-[var(--text-primary)] ${
        isPlay ? "h-dvh overflow-hidden" : "min-h-screen"
      }`}
    >
      <RouteScroll />
      <div className="h-0.5 bg-[var(--honey)]" aria-hidden />
      <header className="sticky top-0 z-50 border-b border-[var(--line)] bg-[var(--bg-page)]">
        <nav className="site-shell" aria-label="Primary">
          <div className="flex h-14 items-center justify-between gap-4">
            <Link to="/" className="flex items-center gap-2.5">
              <AmberLogo className="h-6 w-6" />
              <span className="text-[15px] font-semibold tracking-tight">
                Amber
              </span>
              <span className="hidden font-mono text-xs text-[var(--honey-text)] sm:inline">
                {AMBER_VERSION}
              </span>
            </Link>

            <div className="hidden items-center gap-1 text-sm md:flex">
              {NAV_LINKS.map((item) => {
                const active = isActive(location.pathname, item.to);
                return (
                  <Link
                    key={item.to}
                    to={item.to}
                    aria-current={active ? "page" : undefined}
                    className={`px-3 py-1.5 ${
                      active
                        ? "font-medium text-[var(--text-primary)] underline decoration-[var(--honey)] decoration-2 underline-offset-[6px]"
                        : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                    }`}
                  >
                    {copy.nav[item.key]}
                  </Link>
                );
              })}
            </div>

            <div className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={toggleNext}
                className="inline-flex items-center gap-1.5 px-2 py-1 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                title={copy.theme.toggle}
                aria-label={copy.theme.toggle}
              >
                {theme === "system" && <Monitor className="h-4 w-4" />}
                {theme === "light" && <Sun className="h-4 w-4" />}
                {theme === "dark" && <Moon className="h-4 w-4" />}
                <span className="hidden lg:inline">
                  {theme === "system"
                    ? copy.theme.system
                    : theme === "light"
                      ? copy.theme.light
                      : copy.theme.dark}
                </span>
              </button>

              <LanguageSelector />

              <a
                href={REPO_URL}
                target="_blank"
                rel="noreferrer"
                className="btn-ink !px-3 !py-1.5 !text-xs"
              >
                <Github className="h-4 w-4" />
                <span className="hidden sm:inline">{copy.nav.github}</span>
              </a>

              <button
                type="button"
                onClick={() => setMenuPath(menuOpen ? null : location.pathname)}
                className="inline-flex items-center p-1.5 text-[var(--text-secondary)] hover:text-[var(--text-primary)] md:hidden"
                aria-label={copy.nav.menu}
                aria-expanded={menuOpen}
                aria-controls="mobile-nav"
              >
                {menuOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
              </button>
            </div>
          </div>
          {menuOpen && (
            <div
              id="mobile-nav"
              className="flex flex-col border-t border-[var(--line)] py-2 text-sm md:hidden"
            >
              <Link
                to="/"
                className="py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              >
                {copy.nav.home}
              </Link>
              {NAV_LINKS.map((item) => (
                <Link
                  key={item.to}
                  to={item.to}
                  aria-current={
                    isActive(location.pathname, item.to) ? "page" : undefined
                  }
                  className={`py-2 ${
                    isActive(location.pathname, item.to)
                      ? "font-medium text-[var(--text-primary)]"
                      : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  {copy.nav[item.key]}
                </Link>
              ))}
            </div>
          )}
        </nav>
      </header>

      <main className={isPlay ? "flex min-h-0 grow flex-col" : "grow"}>
        <Outlet />
      </main>

      {isPlay ? null : (
        <footer className="mt-16 border-t border-[var(--line)]">
          <div className="site-shell grid gap-10 py-12 text-sm md:grid-cols-[1.4fr_repeat(3,1fr)]">
            <div className="max-w-xs">
              <div className="flex items-center gap-2">
                <AmberLogo className="h-5 w-5" />
                <span className="font-semibold">Amber</span>
                <span className="font-mono text-xs text-[var(--honey-text)]">
                  {AMBER_VERSION}
                </span>
              </div>
              <p className="mt-3 text-[var(--text-muted)]">
                {copy.footer.tagline}
              </p>
            </div>
            {footerColumns.map((column) => (
              <div key={column.title}>
                <div className="meta uppercase tracking-wider">
                  {column.title}
                </div>
                <ul className="mt-3 space-y-2">
                  {column.links.map((link) => (
                    <li key={link.label}>
                      {"to" in link && link.to ? (
                        <Link
                          to={link.to}
                          className="text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:underline"
                        >
                          {link.label}
                        </Link>
                      ) : (
                        <a
                          href={"href" in link ? link.href : undefined}
                          target="_blank"
                          rel="noreferrer"
                          className="text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:underline"
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
            className="site-shell grid gap-6 border-t border-[var(--line)] py-8 text-sm md:grid-cols-[1.4fr_3fr]"
          >
            <div className="max-w-xs">
              <h2 id="footer-agents" className="meta uppercase tracking-wider">
                {copy.footer.agents.title}
              </h2>
              <p className="mt-3 text-[var(--text-muted)]">
                {copy.footer.agents.intro}
              </p>
            </div>
            <div className="grid gap-5">
              <ul className="flex flex-wrap gap-x-5 gap-y-2 font-mono text-[13px]">
                {AGENT_DOCS.map((doc) => (
                  <li key={doc.href}>
                    <a
                      href={doc.href}
                      className="text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:underline"
                    >
                      {copy.footer.agents[doc.key]}
                    </a>
                  </li>
                ))}
              </ul>
              <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
                <span className="text-[var(--text-muted)]">
                  {copy.footer.agents.ask}
                </span>
                {ASK_AI.map((ai) => (
                  <a
                    key={ai.name}
                    href={`${ai.base}${encodeURIComponent(copy.footer.agents.prompt)}`}
                    target="_blank"
                    rel="noreferrer"
                    className="rounded-md border border-[var(--line)] px-2.5 py-1 text-xs font-medium text-[var(--text-secondary)] hover:border-[var(--text-primary)] hover:text-[var(--text-primary)]"
                  >
                    {ai.name}
                  </a>
                ))}
              </div>
            </div>
          </section>
          <div className="border-t border-[var(--line)]">
            <div className="site-shell flex flex-col gap-2 py-5 text-xs text-[var(--text-muted)] sm:flex-row sm:justify-between">
              <span>© {new Date().getFullYear()} Amber contributors · MIT</span>
              <span className="font-mono">amberjs.com</span>
            </div>
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
        <RootLayoutInner />
      </LangProvider>
    </ThemeProvider>
  );
}
