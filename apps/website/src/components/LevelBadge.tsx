import { useLang } from "../lib/i18n";
import type { Level } from "../lib/locales/types";

const DOT: Record<Level, string> = {
  stable: "bg-emerald-500",
  preview: "bg-[var(--honey)]",
  experimental: "bg-sky-500",
  nongoal: "bg-rose-500",
};

export function LevelBadge({
  level,
  className = "",
}: {
  level: Level;
  className?: string;
}) {
  const { copy } = useLang();
  return (
    <span
      className={`inline-flex shrink-0 items-center gap-1.5 border border-[var(--line)] px-1.5 py-1 font-mono text-xs leading-none text-[var(--text-secondary)] ${className}`}
    >
      <span className={`h-1.5 w-1.5 rounded-full ${DOT[level]}`} aria-hidden />
      {copy.levels[level].label}
    </span>
  );
}
