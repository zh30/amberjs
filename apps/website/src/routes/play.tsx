import { useEffect, useState, type ComponentType } from "react";
import { useLang } from "../lib/i18n";

export default function PlayPage() {
  const { copy } = useLang();
  const [Editor, setEditor] = useState<ComponentType | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void import("../playground/editor")
      .then((mod) => {
        if (!cancelled) setEditor(() => mod.PlaygroundEditor);
      })
      .catch((err: unknown) => {
        if (!cancelled)
          setError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return (
      <div className="page">
        <p className="text-[var(--text-muted)]">{error}</p>
      </div>
    );
  }

  if (!Editor) {
    return (
      <div className="page">
        <p className="text-[var(--text-muted)]">{copy.playground.loading}</p>
      </div>
    );
  }

  return <Editor />;
}
