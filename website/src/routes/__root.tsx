import { useState, useRef, useEffect } from "react";
import { Link, Outlet } from "react-router-dom";
import { BeeLogo } from "../components/Logo";
import "../global.css";
import { LangProvider, useLang } from "../lib/i18n";
import { BEEJS_VERSION } from "../lib/version";
import { ThemeProvider, useTheme } from "../lib/theme";
import {
  Check,
  ChevronDown,
  Github,
  Globe,
  Monitor,
  Moon,
  Sparkles,
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
        className="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-mono text-zinc-600 dark:text-zinc-400 hover:text-zinc-950 dark:hover:text-white hover:bg-zinc-200/60 dark:hover:bg-zinc-800/50 border border-transparent hover:border-zinc-300 dark:hover:border-zinc-700/50 transition-all cursor-pointer"
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
        <ChevronDown
          className={`w-3 h-3 text-zinc-400 dark:text-zinc-500 transition-transform duration-200 ${
            isOpen ? "rotate-180 text-amber-500" : ""
          }`}
        />
      </button>

      {isOpen && (
        <div
          role="listbox"
          aria-label={copy.toggle.label}
          className="absolute right-0 top-full mt-2 w-48 rounded-2xl p-1.5 shadow-2xl border border-zinc-200/80 dark:border-zinc-800/90 bg-white/95 dark:bg-[#0c0d12]/95 backdrop-blur-xl z-50 animate-in fade-in zoom-in-95 duration-150"
        >
          <div className="px-2.5 py-1.5 text-[10px] font-mono uppercase tracking-wider text-zinc-400 dark:text-zinc-500 border-b border-zinc-200/50 dark:border-zinc-800/50 mb-1">
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
                className={`w-full flex items-center justify-between px-2.5 py-2 rounded-xl text-xs font-sans transition-all cursor-pointer ${
                  isSelected
                    ? "bg-amber-500/10 text-amber-600 dark:text-amber-400 font-semibold"
                    : "text-zinc-700 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800/60 hover:text-zinc-950 dark:hover:text-white"
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm leading-none">{option.flag}</span>
                  <span className="font-medium">{option.nativeLabel}</span>
                  <span className="text-[10px] font-mono text-zinc-400 dark:text-zinc-500">
                    ({option.label})
                  </span>
                </div>
                {isSelected && (
                  <Check className="w-3.5 h-3.5 text-amber-500 shrink-0 ml-2" />
                )}
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

  return (
    <div className="min-h-screen flex flex-col relative overflow-x-clip bg-[var(--bg-page)] text-[var(--text-primary)] font-sans selection:bg-amber-500/25 selection:text-amber-950 dark:selection:text-amber-200 transition-colors duration-200">
      {/* Background Ambient Lights Container */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none z-0">
        <div className="ambient-glow top-[-100px] left-1/2 -translate-x-1/2 w-[600px] h-[350px] bg-amber-500/10 dark:bg-amber-500/15" />
        <div className="ambient-glow top-[400px] right-[-150px] w-[500px] h-[400px] bg-indigo-500/5 dark:bg-indigo-500/10" />
      </div>

      {/* Floating Header */}
      <header className="sticky top-4 z-50 max-w-6xl mx-auto w-[calc(100%-2rem)] my-2">
        <nav className="glass-panel rounded-full px-5 py-3 flex items-center justify-between transition-all duration-300">
          <Link to="/" className="flex items-center gap-3 group">
            <BeeLogo className="w-7 h-7 transition-transform group-hover:scale-105" />
            <span className="text-lg font-bold tracking-tight text-zinc-950 dark:text-white font-display">
              BEEJS
            </span>
            <span className="hidden sm:inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20 shadow-sm shadow-amber-500/10">
              <Sparkles className="w-2.5 h-2.5" /> {BEEJS_VERSION}
            </span>
          </Link>

          <div className="hidden md:flex items-center gap-8 text-sm font-medium text-zinc-600 dark:text-zinc-400">
            <Link
              to="/"
              className="hover:text-zinc-950 dark:hover:text-white transition-colors"
            >
              {copy.nav.home}
            </Link>
            <Link
              to="/docs"
              className="hover:text-zinc-950 dark:hover:text-white transition-colors"
            >
              {copy.nav.docs}
            </Link>
            <Link
              to="/blog"
              className="hover:text-zinc-950 dark:hover:text-white transition-colors"
            >
              {copy.nav.blog}
            </Link>
          </div>

          <div className="flex items-center gap-2 sm:gap-3">
            {/* Theme Toggle Button */}
            <button
              type="button"
              onClick={toggleNext}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-mono text-zinc-600 dark:text-zinc-400 hover:text-zinc-950 dark:hover:text-white hover:bg-zinc-200/60 dark:hover:bg-zinc-800/50 border border-transparent hover:border-zinc-300 dark:hover:border-zinc-700/50 transition-all cursor-pointer"
              title={copy.theme.toggle}
              aria-label={copy.theme.toggle}
            >
              {theme === "system" && (
                <Monitor className="w-3.5 h-3.5 text-amber-500 shrink-0" />
              )}
              {theme === "light" && (
                <Sun className="w-3.5 h-3.5 text-amber-500 shrink-0" />
              )}
              {theme === "dark" && (
                <Moon className="w-3.5 h-3.5 text-amber-400 shrink-0" />
              )}
              <span className="hidden sm:inline text-[11px] font-medium">
                {theme === "system"
                  ? copy.theme.system
                  : theme === "light"
                    ? copy.theme.light
                    : copy.theme.dark}
              </span>
            </button>

            {/* Modern Language Selector Dropdown */}
            <LanguageSelector />

            <a
              href="https://github.com/zh30/beejs"
              target="_blank"
              rel="noreferrer"
              className="flex items-center gap-2 px-3.5 sm:px-4 py-1.5 rounded-full bg-zinc-900 text-white dark:bg-white dark:text-zinc-950 font-semibold text-xs hover:bg-zinc-800 dark:hover:bg-zinc-200 transition-all shadow-sm"
            >
              <Github className="w-4 h-4" />
              <span className="hidden sm:inline">{copy.nav.github}</span>
            </a>
          </div>
        </nav>
      </header>

      <main className="grow relative z-10">
        <Outlet />
      </main>

      {/* Sleek Minimalist Footer */}
      <footer className="relative z-10 border-t border-zinc-200/80 dark:border-zinc-800/50 bg-white/80 dark:bg-[#07080a]/80 backdrop-blur-md mt-20">
        <div className="max-w-6xl mx-auto px-6 py-12 flex flex-col md:flex-row items-center justify-between gap-6 text-xs text-zinc-600 dark:text-zinc-400">
          <div className="flex items-center gap-3">
            <BeeLogo className="w-5 h-5 opacity-90" />
            <span className="text-zinc-900 dark:text-zinc-200 font-semibold font-display">
              Beejs Runtime
            </span>
            <span className="text-zinc-400 dark:text-zinc-600">•</span>
            <span>{copy.footer.builtWith}</span>
          </div>

          <div className="flex items-center gap-6 font-medium">
            <Link
              to="/docs"
              className="hover:text-zinc-950 dark:hover:text-white transition-colors"
            >
              {copy.footer.docs}
            </Link>
            <Link
              to="/blog"
              className="hover:text-zinc-950 dark:hover:text-white transition-colors"
            >
              {copy.footer.blog}
            </Link>
            <a
              href="https://github.com/zh30/beejs"
              target="_blank"
              rel="noreferrer"
              className="hover:text-zinc-950 dark:hover:text-white transition-colors"
            >
              {copy.footer.githubRepo}
            </a>
          </div>

          <div className="text-zinc-500 dark:text-zinc-500">
            {copy.footer.copyright}
          </div>
        </div>
      </footer>
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
