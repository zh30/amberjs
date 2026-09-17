import { useCallback, useEffect, useRef, useState } from "react";
import { useTheme } from "../lib/theme";
import { useLang } from "../lib/i18n";
import "./monaco-env";
import * as monaco from "monaco-editor";
import "../../node_modules/monaco-editor/min/vs/editor/editor.main.css";

const SAMPLE_TS = `const runtime = "Amber playground";

function greet(name: string): string {
  return \`hello from \${runtime}, \${name}\`;
}

console.log(greet("world"));
console.log({ n: 1 + 1, ok: true });
`;

const SAMPLE_JS = `const runtime = "Amber playground";

function greet(name) {
  return "hello from " + runtime + ", " + name;
}

console.log(greet("world"));
console.log({ n: 1 + 1, ok: true });
`;

type LangId = "typescript" | "javascript";

function iframeSrc(): string {
  const html = `<!doctype html><html><head><meta charset="utf-8"></head><body>
<script>
(function () {
  function send(msg) { parent.postMessage(msg, "*"); }
  const orig = {
    log: console.log.bind(console),
    error: console.error.bind(console),
    warn: console.warn.bind(console),
  };
  function wrap(level) {
    console[level] = function () {
      const args = Array.prototype.slice.call(arguments).map(function (v) {
        try { return typeof v === "string" ? v : JSON.stringify(v); }
        catch (e) { return String(v); }
      });
      send({ type: "log", level: level, text: args.join(" ") });
      orig[level].apply(console, arguments);
    };
  }
  wrap("log"); wrap("error"); wrap("warn");
  window.onerror = function (m) { send({ type: "error", text: String(m) }); };
  window.addEventListener("message", function (ev) {
    if (!ev.data || ev.data.type !== "run") return;
    try {
      var result = (0, eval)(ev.data.code);
      if (result !== undefined) send({ type: "log", level: "log", text: String(result) });
      send({ type: "done" });
    } catch (err) {
      send({ type: "error", text: err && err.stack ? String(err.stack) : String(err) });
      send({ type: "done" });
    }
  });
})();
</script></body></html>`;
  return URL.createObjectURL(new Blob([html], { type: "text/html" }));
}

export function PlaygroundEditor() {
  const { copy } = useLang();
  const { resolvedTheme } = useTheme();
  const hostRef = useRef<HTMLDivElement | null>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const [lang, setLang] = useState<LangId>("typescript");
  const [running, setRunning] = useState(false);
  const [frameReady, setFrameReady] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const [runnerSrc] = useState(() => iframeSrc());

  useEffect(() => {
    return () => {
      URL.revokeObjectURL(runnerSrc);
    };
  }, [runnerSrc]);

  useEffect(() => {
    if (!hostRef.current) return;

    monaco.typescript.typescriptDefaults.setCompilerOptions({
      target: monaco.typescript.ScriptTarget.ESNext,
      allowNonTsExtensions: true,
      moduleResolution: monaco.typescript.ModuleResolutionKind.NodeJs,
      module: monaco.typescript.ModuleKind.ESNext,
      jsx: monaco.typescript.JsxEmit.React,
      strict: true,
      noEmit: false,
      lib: ["es2022", "dom"],
    });
    monaco.typescript.javascriptDefaults.setCompilerOptions({
      target: monaco.typescript.ScriptTarget.ESNext,
      allowNonTsExtensions: true,
      checkJs: true,
      noEmit: false,
    });

    const editor = monaco.editor.create(hostRef.current, {
      value: SAMPLE_TS,
      language: "typescript",
      theme: resolvedTheme === "dark" ? "vs-dark" : "vs",
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      fontFamily: "JetBrains Mono, ui-monospace, monospace",
      tabSize: 2,
      scrollBeyondLastLine: false,
      padding: { top: 12 },
      renderLineHighlight: "line",
    });
    editorRef.current = editor;
    return () => {
      editor.dispose();
      editorRef.current = null;
    };
  }, []);

  useEffect(() => {
    monaco.editor.setTheme(resolvedTheme === "dark" ? "vs-dark" : "vs");
  }, [resolvedTheme]);

  const onLang = (next: LangId) => {
    setLang(next);
    const editor = editorRef.current;
    if (!editor) return;
    const model = editor.getModel();
    if (!model) return;
    monaco.editor.setModelLanguage(model, next);
    editor.setValue(next === "typescript" ? SAMPLE_TS : SAMPLE_JS);
  };

  const run = useCallback(async () => {
    const editor = editorRef.current;
    const frame = iframeRef.current;
    if (!editor || !frame || !frame.contentWindow) return;
    setRunning(true);
    setLines([]);

    const model = editor.getModel();
    if (!model) {
      setRunning(false);
      return;
    }

    let code = editor.getValue();
    if (lang === "typescript") {
      const worker = await monaco.typescript.getTypeScriptWorker();
      const client = await worker(model.uri);
      const emitted = await client.getEmitOutput(model.uri.toString());
      const file = emitted.outputFiles[0];
      if (file) code = file.text;
    }

    const onMsg = (ev: MessageEvent) => {
      const data = ev.data as { type?: string; text?: string; level?: string };
      if (!data || typeof data.type !== "string") return;
      if (data.type === "log" && data.text) {
        setLines((prev) => [...prev, data.text ?? ""]);
      } else if (data.type === "error" && data.text) {
        setLines((prev) => [...prev, "error: " + data.text]);
      } else if (data.type === "done") {
        window.removeEventListener("message", onMsg);
        setRunning(false);
      }
    };
    window.addEventListener("message", onMsg);
    frame.contentWindow.postMessage({ type: "run", code }, "*");
    window.setTimeout(() => {
      window.removeEventListener("message", onMsg);
      setRunning(false);
    }, 8000);
  }, [lang]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-3 border-b border-[var(--line)] px-4 py-3">
        <h1 className="text-lg font-semibold">{copy.playground.title}</h1>
        <label className="ml-auto flex items-center gap-2 text-sm">
          <span className="text-[var(--text-muted)]">
            {copy.playground.language}
          </span>
          <select
            value={lang}
            onChange={(e) => onLang(e.target.value as LangId)}
            className="border border-[var(--line)] bg-[var(--bg-page)] px-2 py-1"
          >
            <option value="typescript">TypeScript</option>
            <option value="javascript">JavaScript</option>
          </select>
        </label>
        <button
          type="button"
          onClick={() => void run()}
          disabled={running || !frameReady}
          className="btn-honey disabled:opacity-60"
        >
          {running ? copy.playground.running : copy.playground.run}
        </button>
      </div>
      <p className="border-b border-[var(--line)] px-4 py-2 text-sm text-[var(--text-muted)]">
        {copy.playground.note}
      </p>
      <div className="grid min-h-0 flex-1 lg:grid-cols-[1.4fr_0.8fr]">
        <div ref={hostRef} className="min-h-[320px] min-w-0" />
        <div className="flex min-h-[200px] flex-col border-t border-[var(--line)] lg:border-t-0 lg:border-l">
          <div className="border-b border-[var(--line)] px-4 py-2 font-mono text-xs text-[var(--text-muted)]">
            {copy.playground.output}
          </div>
          <pre className="min-h-0 flex-1 overflow-auto bg-[var(--code-bg)] p-4 font-mono text-[13px] leading-relaxed text-[var(--code-fg)]">
            {lines.length === 0 ? copy.playground.empty : lines.join("\n")}
          </pre>
        </div>
      </div>
      <iframe
        ref={iframeRef}
        title="playground-runner"
        sandbox="allow-scripts"
        src={runnerSrc}
        className="hidden"
        onLoad={() => setFrameReady(true)}
      />
    </div>
  );
}
