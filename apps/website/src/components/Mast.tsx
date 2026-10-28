import { useEffect, useRef, useState, type ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";
import { Check, ChevronDown, Github } from "lucide-react";
import { AmberLogo } from "./Logo";
import { useLang } from "../lib/i18n";
import { landingCopy } from "../lib/landing-copy";
import { REPO_URL } from "../lib/scope";
import { useTheme } from "../lib/theme";
import { AMBER_VERSION } from "../lib/version";

const NAV = [
  { to: "/", key: "home" },
  { to: "/docs", key: "docs" },
  { to: "/compatibility", key: "scope" },
  { to: "/blog", key: "blog" },
  { to: "/play", key: "play" },
] as const;

function isActive(pathname: string, to: string) {
  if (to === "/") return pathname === "/";
  return pathname === to || pathname.startsWith(`${to}/`);
}

function LanguageSelector() {
  const { lang, setLang, copy, languages } = useLang();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const current = languages.find((item) => item.code === lang) ?? languages[0];

  useEffect(() => {
    if (!open) return;
    function onPointer(event: MouseEvent | TouchEvent) {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", onPointer);
    document.addEventListener("touchstart", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      document.removeEventListener("touchstart", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        className="textbtn"
        onClick={() => setOpen((value) => !value)}
        aria-label={copy.toggle.label}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        {current.nativeLabel}
        <ChevronDown className={`h-3.5 w-3.5 ${open ? "rotate-180" : ""}`} />
      </button>
      {open ? (
        <div className="lang-menu" role="listbox" aria-label={copy.toggle.label}>
          {languages.map((option) => {
            const selected = option.code === lang;
            return (
              <button
                key={option.code}
                type="button"
                role="option"
                aria-selected={selected}
                className="flex w-full items-center justify-between px-3 py-2 text-left text-sm"
                onClick={() => {
                  setLang(option.code);
                  setOpen(false);
                }}
              >
                <span>
                  {option.nativeLabel}
                  <span className="ml-2 font-mono text-xs text-[var(--muted)]">
                    {option.label}
                  </span>
                </span>
                {selected ? <Check className="h-4 w-4" /> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

export function Mast({ trailing }: { trailing?: ReactNode }) {
  const { copy, lang, setLang, languages } = useLang();
  const { theme, toggleNext } = useTheme();
  const location = useLocation();
  const [openPath, setOpenPath] = useState<string | null>(null);
  const open = openPath === location.pathname;
  const text = landingCopy[lang];
  const themeLabel =
    theme === "system"
      ? copy.theme.system
      : theme === "light"
        ? copy.theme.light
        : copy.theme.dark;

  useEffect(() => {
    setOpenPath(null);
  }, [location.pathname]);

  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") setOpenPath(null);
    }
    document.addEventListener("keydown", onKey);
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = previous;
    };
  }, [open]);

  return (
    <>
      <header className="mast left">
        <Link to="/" className="brand">
          <AmberLogo className="h-9 w-9" />
          <span>Amber</span>
        </Link>
        <nav className="desk" aria-label="Primary">
          {NAV.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              aria-current={isActive(location.pathname, item.to) ? "page" : undefined}
            >
              {copy.nav[item.key]}
            </Link>
          ))}
        </nav>
        <button
          type="button"
          className="burger"
          aria-label={copy.nav.menu}
          aria-expanded={open}
          aria-controls="mobile-nav"
          onClick={() => setOpenPath(open ? null : location.pathname)}
        >
          {copy.nav.menu}
        </button>
        <div className={`panel${open ? " open" : ""}`} id="mobile-nav">
          <button type="button" onClick={() => setOpenPath(null)}>
            {text.close}
          </button>
          {NAV.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              aria-current={isActive(location.pathname, item.to) ? "page" : undefined}
            >
              {copy.nav[item.key]}
            </Link>
          ))}
          <button type="button" onClick={toggleNext}>
            {themeLabel}
          </button>
          {languages.map((option) => (
            <button
              key={option.code}
              type="button"
              aria-current={option.code === lang ? "true" : undefined}
              onClick={() => setLang(option.code)}
            >
              {option.nativeLabel}
            </button>
          ))}
          <a href={REPO_URL} target="_blank" rel="noreferrer">
            {copy.nav.github}
          </a>
        </div>
      </header>
      <header className="mast right">
        {trailing}
        <div className="mast-right-end">
          <div className="mast-tools">
            <button
              type="button"
              className="textbtn"
              onClick={toggleNext}
              aria-label={copy.theme.toggle}
            >
              {themeLabel}
            </button>
            <LanguageSelector />
            <a className="textbtn" href={REPO_URL} target="_blank" rel="noreferrer">
              <Github className="h-4 w-4" />
              {copy.nav.github}
            </a>
          </div>
          <p className="ver">{AMBER_VERSION}</p>
        </div>
      </header>
    </>
  );
}
