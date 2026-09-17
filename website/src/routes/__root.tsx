import { useState, useRef, useEffect } from "react";
import { Link, Outlet, useLocation } from "react-router-dom";
import { BeeLogo } from "../components/Logo";
import "../global.css";
import { LangProvider, useLang } from "../lib/i18n";
import { RouteScroll } from "../lib/route-scroll";
import { BEEJS_VERSION } from "../lib/version";
import { ThemeProvider, useTheme } from "../lib/theme";
import {
  Check,
  ChevronDown,
  Github,
  Globe,
  Monitor,
  Moon,
  Sun,
} from "lucide-react";

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

function RootLayoutInner() {
  const { copy } = useLang();
  const { theme, toggleNext } = useTheme();
  const location = useLocation();
  const isPlay = location.pathname === "/play";

  return (
    <div
      className={`flex flex-col bg-[var(--bg-page)] text-[var(--text-primary)] ${
        isPlay ? "h-dvh overflow-hidden" : "min-h-screen"
      }`}
    >
      <RouteScroll />
      <div className="h-0.5 bg-[var(--honey)]" aria-hidden />
      <header className="sticky top-0 z-50 border-b border-[var(--line)] bg-[var(--bg-page)]">
        <nav className="site-shell">
          <div className="flex h-14 items-center justify-between">
            <Link to="/" className="flex items-center gap-2.5">
              <BeeLogo className="h-6 w-6" />
              <span className="text-[15px] font-semibold tracking-tight">
                Beejs
              </span>
              <span className="hidden font-mono text-xs text-[var(--honey-text)] sm:inline">
                {BEEJS_VERSION}
              </span>
            </Link>

            <div className="hidden items-center gap-6 text-sm text-[var(--text-secondary)] md:flex">
              <Link
                to="/"
                className="hover:text-[var(--text-primary)] hover:underline"
              >
                {copy.nav.home}
              </Link>
              <Link
                to="/docs"
                className="hover:text-[var(--text-primary)] hover:underline"
              >
                {copy.nav.docs}
              </Link>
              <Link
                to="/blog"
                className="hover:text-[var(--text-primary)] hover:underline"
              >
                {copy.nav.blog}
              </Link>
              <Link
                to="/play"
                className="hover:text-[var(--text-primary)] hover:underline"
              >
                {copy.nav.play}
              </Link>
            </div>

            <div className="flex items-center gap-2">
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
                <span className="hidden sm:inline">
                  {theme === "system"
                    ? copy.theme.system
                    : theme === "light"
                      ? copy.theme.light
                      : copy.theme.dark}
                </span>
              </button>

              <LanguageSelector />

              <a
                href="https://github.com/zh30/beejs"
                target="_blank"
                rel="noreferrer"
                className="btn-ink !px-3 !py-1.5 !text-xs"
              >
                <Github className="h-4 w-4" />
                <span className="hidden sm:inline">{copy.nav.github}</span>
              </a>
            </div>
          </div>
          <div className="flex gap-5 pb-3 text-sm text-[var(--text-secondary)] md:hidden">
            <Link
              to="/"
              className="hover:text-[var(--text-primary)] hover:underline"
            >
              {copy.nav.home}
            </Link>
            <Link
              to="/docs"
              className="hover:text-[var(--text-primary)] hover:underline"
            >
              {copy.nav.docs}
            </Link>
            <Link
              to="/blog"
              className="hover:text-[var(--text-primary)] hover:underline"
            >
              {copy.nav.blog}
            </Link>
            <Link
              to="/play"
              className="hover:text-[var(--text-primary)] hover:underline"
            >
              {copy.nav.play}
            </Link>
          </div>
        </nav>
      </header>

      <main className={isPlay ? "flex min-h-0 grow flex-col" : "grow"}>
        <Outlet />
      </main>

      {isPlay ? null : (
        <footer className="mt-16 border-t border-[var(--line)]">
          <div className="site-shell flex flex-col gap-4 py-8 text-sm text-[var(--text-muted)] md:flex-row md:items-center md:justify-between">
            <div className="flex items-center gap-2">
              <BeeLogo className="h-4 w-4" />
              <span>{copy.footer.builtWith}</span>
            </div>
            <div className="flex gap-5">
              <Link
                to="/docs"
                className="hover:text-[var(--text-primary)] hover:underline"
              >
                {copy.footer.docs}
              </Link>
              <Link
                to="/blog"
                className="hover:text-[var(--text-primary)] hover:underline"
              >
                {copy.footer.blog}
              </Link>
              <a
                href="https://github.com/zh30/beejs"
                target="_blank"
                rel="noreferrer"
                className="hover:text-[var(--text-primary)] hover:underline"
              >
                {copy.footer.githubRepo}
              </a>
            </div>
            <div>{copy.footer.copyright}</div>
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
