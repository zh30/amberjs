import { useState, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";

/** Shell prompts start with "$ ", comments with "//" or "#". Everything else after a prompt is output. */
function renderLines(source: string): ReactNode[] {
  let afterPrompt = false;
  return source.split("\n").map((line, i) => {
    if (line.startsWith("$ ")) {
      afterPrompt = true;
      return (
        <div key={i}>
          <span className="select-none text-[var(--honey)]">$ </span>
          <span className="text-white">{line.slice(2)}</span>
        </div>
      );
    }
    if (afterPrompt && line.startsWith("  ")) {
      return (
        <div key={i} className="text-white">
          {line}
        </div>
      );
    }
    if (line.trim() === "") {
      afterPrompt = false;
      return <div key={i}>&nbsp;</div>;
    }
    if (/^\s*(\/\/|#)/.test(line)) {
      return (
        <div key={i} className="text-zinc-500">
          {line}
        </div>
      );
    }
    return (
      <div key={i} className={afterPrompt ? "text-zinc-400" : undefined}>
        {line}
      </div>
    );
  });
}

export function CodeWindow({
  title,
  code,
  copyText,
  copyLabel = "Copy",
  copiedLabel = "Copied",
  className = "",
}: {
  title: string;
  code: string;
  copyText?: string;
  copyLabel?: string;
  copiedLabel?: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);
  const text = copyText ?? code;

  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch {
      /* clipboard unavailable */
    }
  };

  return (
    <div
      className={`min-w-0 overflow-hidden rounded-lg border border-black/10 bg-[var(--code-bg)] text-[var(--code-fg)] shadow-[0_1px_0_rgba(0,0,0,0.04),0_12px_32px_-12px_rgba(0,0,0,0.35)] dark:border-white/10 ${className}`}
    >
      <div className="flex items-center justify-between gap-3 border-b border-white/10 px-4 py-2.5">
        <div className="flex items-center gap-3">
          <span className="flex gap-1.5" aria-hidden>
            <span className="h-2.5 w-2.5 rounded-full bg-white/15" />
            <span className="h-2.5 w-2.5 rounded-full bg-white/15" />
            <span className="h-2.5 w-2.5 rounded-full bg-white/15" />
          </span>
          <span className="font-mono text-[12px] text-zinc-400">{title}</span>
        </div>
        <button
          type="button"
          onClick={onCopy}
          className="inline-flex items-center gap-1 font-mono text-[11px] text-zinc-400 hover:text-white"
          aria-label={copyLabel}
        >
          {copied ? (
            <>
              <Check className="h-3.5 w-3.5 text-emerald-400" /> {copiedLabel}
            </>
          ) : (
            <>
              <Copy className="h-3.5 w-3.5" /> {copyLabel}
            </>
          )}
        </button>
      </div>
      <pre className="m-0 overflow-x-auto px-4 py-4 font-mono text-[13px] leading-[1.7]">
        <code>{renderLines(code)}</code>
      </pre>
    </div>
  );
}
