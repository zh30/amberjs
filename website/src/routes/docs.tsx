import { Link, useParams } from "react-router-dom";
import {
  useState,
  useEffect,
  useMemo,
  isValidElement,
  type ReactNode,
} from "react";
import {
  Activity,
  Book,
  Code,
  Cpu,
  Layers,
  Terminal,
  Zap,
  Server,
  ArrowLeft,
  ArrowRight,
  Sparkles,
  Package,
  Copy,
  Check,
  Search,
  X,
  Workflow,
  CheckCircle2,
  Box,
  Gauge,
  ShieldCheck,
  Binary,
  FileCode,
  Info,
  Lightbulb,
  AlertCircle,
  AlertTriangle,
  ListTree,
  Database,
  Rocket,
  RotateCcw,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useLang } from "../lib/i18n";

const iconMap: Record<string, ReactNode> = {
  introduction: <Book className="w-4 h-4" />,
  installation: <Terminal className="w-4 h-4" />,
  "quick-start": <Zap className="w-4 h-4" />,
  "v8-isolate-pool": <Cpu className="w-4 h-4" />,
  "jit-optimization": <Activity className="w-4 h-4" />,
  "ai-engine": <Sparkles className="w-4 h-4" />,
  "ai-embeddings": <Sparkles className="w-4 h-4" />,
  "server-mode": <Server className="w-4 h-4" />,
  "memory-management": <Layers className="w-4 h-4" />,
  "embedded-db": <Database className="w-4 h-4" />,
  "standard-library": <Sparkles className="w-4 h-4" />,
  "package-manager-dlx": <Binary className="w-4 h-4" />,
  "deployment-docker": <Rocket className="w-4 h-4" />,
  "ide-extension": <FileCode className="w-4 h-4" />,
  "task-runner": <Workflow className="w-4 h-4" />,
  "code-quality": <CheckCircle2 className="w-4 h-4" />,
  "bundling-compilation": <Box className="w-4 h-4" />,
  "testing-benchmarking": <Gauge className="w-4 h-4" />,
  "debugging-lsp": <Terminal className="w-4 h-4" />,
  "agent-sandbox": <ShieldCheck className="w-4 h-4" />,
  "agent-replay": <RotateCcw className="w-4 h-4" />,
  "model-weights": <Cpu className="w-4 h-4" />,
  "capability-security": <ShieldCheck className="w-4 h-4" />,
  "kv-store": <Database className="w-4 h-4" />,
  "tool-synthesis": <Workflow className="w-4 h-4" />,
  "hardened-sandbox": <ShieldCheck className="w-4 h-4" />,
  "agent-bus": <Activity className="w-4 h-4" />,
  "streaming-grammar": <Sparkles className="w-4 h-4" />,
  "agent-checkpoint": <RotateCcw className="w-4 h-4" />,
  "mcp-protocol": <Layers className="w-4 h-4" />,
  "virtual-fs-sandbox": <ShieldCheck className="w-4 h-4" />,
  "import-maps-native": <Binary className="w-4 h-4" />,
  "types-lsp": <FileCode className="w-4 h-4" />,
  "cli-usage": <Code className="w-4 h-4" />,
  "api-reference": <Book className="w-4 h-4" />,
  "wintertc-compliance": <ShieldCheck className="w-4 h-4" />,
  "ffi-native": <Binary className="w-4 h-4" />,
  "isolate-pool": <Cpu className="w-4 h-4" />,
  "slm-inference": <Sparkles className="w-4 h-4" />,
  "wasm-interop": <Zap className="w-4 h-4" />,
  "framework-compat": <Layers className="w-4 h-4" />,
  modules: <Package className="w-4 h-4" />,
};

const docModules = import.meta.glob("../docs/*.md", {
  query: "?raw",
  eager: true,
  import: "default",
}) as Record<string, string>;

function parseFrontmatter(raw: string): {
  data: Record<string, string>;
  content: string;
} {
  const match = raw.match(/^---\s*([\s\S]*?)\s*---\s*([\s\S]*)$/);
  if (!match) return { data: {}, content: raw };

  const yaml = match[1];
  const content = match[2];
  const data: Record<string, string> = {};

  yaml
    .split("\n")
    .filter(Boolean)
    .forEach((line) => {
      const [key, ...valueParts] = line.split(":");
      if (key && valueParts.length > 0) {
        data[key.trim()] = valueParts
          .join(":")
          .trim()
          .replace(/^["']|["']$/g, "");
      }
    });

  return { data, content };
}

function getDocContent(section: string, lang: string) {
  // 1. Try language-specific file (e.g. .zh.md, .es.md, etc.)
  if (lang !== "en") {
    const langKey = `../docs/${section}.${lang}.md`;
    if (docModules[langKey]) return parseFrontmatter(docModules[langKey]);
  }

  // 2. If lang is 'zh' or fallback to Chinese
  if (lang === "zh") {
    const zhKey = `../docs/${section}.zh.md`;
    if (docModules[zhKey]) return parseFrontmatter(docModules[zhKey]);
  }

  // 3. Try standard English file
  const enKey = `../docs/${section}.md`;
  if (docModules[enKey]) return parseFrontmatter(docModules[enKey]);

  // 4. Fallback to zh file if en missing
  const fallbackZh = `../docs/${section}.zh.md`;
  if (docModules[fallbackZh]) return parseFrontmatter(docModules[fallbackZh]);

  return null;
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fa5]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function extractToc(
  content: string,
): { id: string; title: string; level: number }[] {
  const headingRegex = /^(#{2,3})\s+(.+)$/gm;
  const toc: { id: string; title: string; level: number }[] = [];
  let match;
  while ((match = headingRegex.exec(content)) !== null) {
    const level = match[1].length;
    const title = match[2].trim().replace(/[*`_]/g, "");
    const id = slugify(title);
    if (id) {
      toc.push({ id, title, level });
    }
  }
  return toc;
}

function nodeText(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(nodeText).join("");
  if (isValidElement<{ children?: ReactNode }>(node))
    return nodeText(node.props.children);
  return "";
}

function PreBlock({ children }: { children?: ReactNode }) {
  const [copied, setCopied] = useState(false);

  const extractText = (node: ReactNode): string => nodeText(node);

  const text = extractText(children).trim();

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative my-6 overflow-hidden border border-[var(--line)] bg-[var(--code-bg)]">
      <div className="flex items-center justify-between border-b border-white/10 px-4 py-2.5 font-mono text-[12px] text-[var(--code-fg)]">
        <span className="flex items-center gap-1.5">
          <Terminal className="h-3.5 w-3.5 opacity-70" />
          <span>code</span>
        </span>
        <button
          onClick={handleCopy}
          aria-label="Copy code"
          className="hover:text-white"
        >
          {copied ? (
            <>
              <Check className="w-3.5 h-3.5 text-emerald-400" />
              <span>Copied</span>
            </>
          ) : (
            <>
              <Copy className="w-3.5 h-3.5" />
              <span>Copy</span>
            </>
          )}
        </button>
      </div>
      <pre className="m-0 overflow-x-auto bg-transparent p-5 font-mono text-[13px] leading-relaxed text-[var(--code-fg)]">
        {children}
      </pre>
    </div>
  );
}

function BlockquoteBlock({ children }: { children?: ReactNode }) {
  const extractText = (node: ReactNode): string => nodeText(node);

  const text = extractText(children).trim();

  if (text.includes("[!NOTE]")) {
    return (
      <div className="my-6 border border-[var(--line)] p-4 text-sm">
        <div className="mb-1 flex items-center gap-2 font-semibold">
          <Info className="w-4 h-4" /> NOTE
        </div>
        <div className="[&>p]:m-0">{children}</div>
      </div>
    );
  }
  if (text.includes("[!TIP]")) {
    return (
      <div className="my-6 border border-[var(--line)] p-4 text-sm">
        <div className="mb-1 flex items-center gap-2 font-semibold">
          <Lightbulb className="w-4 h-4" /> TIP
        </div>
        <div className="[&>p]:m-0">{children}</div>
      </div>
    );
  }
  if (text.includes("[!IMPORTANT]")) {
    return (
      <div className="my-6 border border-[var(--line)] p-4 text-sm">
        <div className="mb-1 flex items-center gap-2 font-semibold">
          <AlertCircle className="w-4 h-4" /> IMPORTANT
        </div>
        <div className="[&>p]:m-0">{children}</div>
      </div>
    );
  }
  if (text.includes("[!WARNING]") || text.includes("[!CAUTION]")) {
    return (
      <div className="my-6 border border-[var(--line)] p-4 text-sm">
        <div className="mb-1 flex items-center gap-2 font-semibold">
          <AlertTriangle className="w-4 h-4" /> WARNING
        </div>
        <div className="[&>p]:m-0">{children}</div>
      </div>
    );
  }

  return (
    <blockquote className="my-6 border border-[var(--line)] p-4 text-sm not-italic">
      {children}
    </blockquote>
  );
}

function TableBlock({ children }: { children?: ReactNode }) {
  return (
    <div className="my-6 overflow-x-auto border border-[var(--line)]">
      <table className="w-full text-left border-collapse text-xs sm:text-sm m-0">
        {children}
      </table>
    </div>
  );
}

export default function DocsComponent() {
  const { section = "introduction" } = useParams();
  const { copy, lang } = useLang();
  const manual = copy.docs;

  const [searchQuery, setSearchQuery] = useState("");
  const [activeHeading, setActiveHeading] = useState<string>("");

  const docData = getDocContent(section, lang);

  // Extract table of contents from markdown content
  const toc = useMemo(() => {
    return docData?.content ? extractToc(docData.content) : [];
  }, [docData]);

  // Track active heading on scroll
  useEffect(() => {
    const handleScroll = () => {
      const headings = document.querySelectorAll("h2[id], h3[id]");
      let current = "";
      for (const el of Array.from(headings)) {
        const top = el.getBoundingClientRect().top;
        if (top <= 150) {
          current = el.id;
        }
      }
      setActiveHeading(current);
    };

    window.addEventListener("scroll", handleScroll, { passive: true });
    handleScroll();
    return () => window.removeEventListener("scroll", handleScroll);
  }, [section]);

  // Find active group title
  let currentGroupTitle = manual.title;
  for (const group of manual.groups) {
    if (group.items.some((item) => item.id === section)) {
      currentGroupTitle = group.title;
      break;
    }
  }

  // Flatten items for pagination
  const allItems = useMemo(() => {
    return manual.groups.flatMap((g) => g.items);
  }, [manual.groups]);

  const currentIndex = allItems.findIndex((item) => item.id === section);
  const prevItem = currentIndex > 0 ? allItems[currentIndex - 1] : null;
  const nextItem =
    currentIndex >= 0 && currentIndex < allItems.length - 1
      ? allItems[currentIndex + 1]
      : null;

  // Filter groups according to search query
  const filteredGroups = useMemo(() => {
    if (!searchQuery.trim()) return manual.groups;
    const q = searchQuery.toLowerCase().trim();
    return manual.groups
      .map((g) => ({
        ...g,
        items: g.items.filter(
          (item) =>
            item.label.toLowerCase().includes(q) ||
            item.id.toLowerCase().includes(q),
        ),
      }))
      .filter((g) => g.items.length > 0);
  }, [manual.groups, searchQuery]);

  // Fallback content if no markdown found
  const fallbackContent =
    manual.sections[section as keyof typeof manual.sections] ||
    manual.sections.introduction;

  const title = docData?.data?.title || fallbackContent?.title || section;
  const subtitle = docData?.data?.subtitle || fallbackContent?.subtitle || "";

  return (
    <div className="page">
      <div className="mx-auto max-w-[1360px] px-4 sm:px-6">
        <div className="grid grid-cols-1 lg:grid-cols-[280px_1fr] xl:grid-cols-[280px_1fr_220px] gap-8 xl:gap-10">
          {/* Left Sidebar */}
          <aside className="sticky top-16 h-fit border border-[var(--line)] p-5">
            <Link
              to="/"
              className="mb-5 inline-flex items-center text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:underline"
            >
              <ArrowLeft className="w-3.5 h-3.5 mr-2" /> {manual.backToHome}
            </Link>

            {/* Quick Search Filter */}
            <div className="relative mb-6">
              <Search className="w-3.5 h-3.5 text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={manual.searchPlaceholder || "Search docs..."}
                className="w-full border border-[var(--line)] bg-[var(--bg-page)] py-2 pl-8.5 pr-8 text-xs text-[var(--text-primary)] placeholder-[var(--text-muted)]"
              />
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery("")}
                  aria-label="Clear search"
                  className="absolute right-2.5 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200 p-0.5 cursor-pointer"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              )}
            </div>

            {/* Nav Groups */}
            <div className="space-y-6 max-h-[calc(100vh-220px)] overflow-y-auto pr-1">
              {filteredGroups.map((group) => (
                <div key={group.title}>
                  <h4 className="mb-2.5 text-[11px] font-medium text-[var(--text-muted)]">
                    {group.title}
                  </h4>
                  <div className="space-y-1">
                    {group.items.map((item) => (
                      <Link
                        key={item.id}
                        to={`/docs/${item.id}`}
                        className={`flex items-center justify-between px-2 py-1.5 text-xs ${
                          section === item.id
                            ? "bg-[var(--honey)] font-semibold text-[var(--honey-ink)]"
                            : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                        }`}
                      >
                        <div className="flex items-center gap-2.5 truncate">
                          <span
                            className={
                              section === item.id
                                ? "text-[var(--text-primary)]"
                                : "text-zinc-500 dark:text-zinc-500"
                            }
                          >
                            {iconMap[item.id] || <Book className="w-4 h-4" />}
                          </span>
                          <span className="truncate">{item.label}</span>
                        </div>

                        {item.badge && (
                          <span className="ml-2 shrink-0 font-mono text-[9px] text-[var(--text-muted)]">
                            {item.badge}
                          </span>
                        )}
                      </Link>
                    ))}
                  </div>
                </div>
              ))}

              {filteredGroups.length === 0 && (
                <div className="text-xs text-zinc-500 text-center py-6">
                  No matching documentation pages.
                </div>
              )}
            </div>
          </aside>

          {/* Main Doc Content */}
          <main
            key={section}
            className="min-w-0 border border-[var(--line)] p-6 sm:p-10"
          >
            {/* Header / Breadcrumb */}
            <div className="space-y-4 mb-8">
              <div className="flex flex-wrap items-center gap-2 text-xs text-[var(--text-muted)]">
                <span>{manual.title}</span>
                <span className="text-zinc-400 dark:text-zinc-600">/</span>
                <span>{currentGroupTitle}</span>
                <span className="text-zinc-400 dark:text-zinc-600">/</span>
                <span className="text-zinc-900 dark:text-zinc-200 font-semibold">
                  {title}
                </span>
              </div>

              <h1 className="text-3xl font-bold tracking-tight">{title}</h1>

              {subtitle && (
                <p className="text-base leading-relaxed text-[var(--text-secondary)]">
                  {subtitle}
                </p>
              )}

              <div className="mt-6 h-px w-full bg-[var(--line)]" />
            </div>

            {/* Markdown Body or Fallback */}
            {docData ? (
              <div className="prose prose-bee dark:prose-invert max-w-none prose-a:underline prose-a:underline-offset-2 [&_:not(pre)>code]:rounded-sm [&_:not(pre)>code]:bg-[var(--honey-soft)] [&_:not(pre)>code]:px-1 [&_:not(pre)>code]:before:content-none [&_:not(pre)>code]:after:content-none">
                <ReactMarkdown
                  remarkPlugins={[remarkGfm]}
                  components={{
                    pre: PreBlock,
                    blockquote: BlockquoteBlock,
                    table: TableBlock,
                    h2: ({ children }) => {
                      const id = slugify(String(children));
                      return (
                        <h2
                          id={id}
                          className="mt-10 mb-4 flex scroll-mt-24 items-center gap-2 border-b border-[var(--line)] pb-2 text-2xl font-bold tracking-tight"
                        >
                          <a
                            href={`#${id}`}
                            className="hover:underline text-inherit no-underline"
                          >
                            {children}
                          </a>
                        </h2>
                      );
                    },
                    h3: ({ children }) => {
                      const id = slugify(String(children));
                      return (
                        <h3
                          id={id}
                          className="mt-6 mb-3 scroll-mt-24 text-lg font-bold tracking-tight"
                        >
                          <a
                            href={`#${id}`}
                            className="hover:underline text-inherit no-underline"
                          >
                            {children}
                          </a>
                        </h3>
                      );
                    },
                  }}
                >
                  {docData.content}
                </ReactMarkdown>
              </div>
            ) : (
              <div className="space-y-6">
                <p className="text-sm text-zinc-700 dark:text-zinc-300 leading-relaxed">
                  {Array.isArray(fallbackContent?.body)
                    ? fallbackContent.body.join("\n\n")
                    : fallbackContent?.body}
                </p>

                {fallbackContent?.list && (
                  <ul className="space-y-3 my-6">
                    {fallbackContent.list.map((item) => (
                      <li
                        key={item}
                        className="flex items-start gap-3 text-sm text-zinc-800 dark:text-zinc-200"
                      >
                        <span className="mt-2 h-1.5 w-1.5 shrink-0 bg-[var(--text-primary)]" />
                        <span>{item}</span>
                      </li>
                    ))}
                  </ul>
                )}

                {fallbackContent?.code && (
                  <div className="terminal overflow-x-auto p-5 font-mono text-xs leading-relaxed">
                    <pre>
                      {Array.isArray(fallbackContent.code)
                        ? fallbackContent.code.join("\n")
                        : fallbackContent.code}
                    </pre>
                  </div>
                )}
              </div>
            )}

            {/* Pagination Cards (Previous / Next) */}
            <div className="mt-14 grid grid-cols-1 gap-4 border-t border-[var(--line)] pt-8 sm:grid-cols-2">
              {prevItem ? (
                <Link
                  to={`/docs/${prevItem.id}`}
                  className="group flex flex-col border border-[var(--line)] p-4"
                >
                  <span className="meta mb-1 flex items-center gap-1">
                    <ArrowLeft className="w-3 h-3 group-hover:-translate-x-1 transition-transform" />
                    {manual.previousPage || "Previous"}
                  </span>
                  <span className="truncate text-sm font-semibold">
                    {prevItem.label}
                  </span>
                </Link>
              ) : (
                <div />
              )}

              {nextItem ? (
                <Link
                  to={`/docs/${nextItem.id}`}
                  className="group flex flex-col items-end border border-[var(--line)] p-4 text-right"
                >
                  <span className="meta mb-1 flex items-center gap-1">
                    {manual.nextPage || "Next"}
                    <ArrowRight className="w-3 h-3 group-hover:translate-x-1 transition-transform" />
                  </span>
                  <span className="truncate text-sm font-semibold">
                    {nextItem.label}
                  </span>
                </Link>
              ) : (
                <div />
              )}
            </div>
          </main>

          {/* Right Sidebar: On This Page (TOC) */}
          <aside className="hidden xl:block">
            <div className="sticky top-24 space-y-4">
              <div className="border border-[var(--line)] p-5">
                <div className="meta mb-3 flex items-center gap-2 font-medium">
                  <ListTree className="h-3.5 w-3.5" />
                  <span>{manual.onThisPage || "On this page"}</span>
                </div>

                {toc.length > 0 ? (
                  <nav className="space-y-1 max-h-[calc(100vh-250px)] overflow-y-auto text-xs">
                    {toc.map((item) => (
                      <a
                        key={item.id}
                        href={`#${item.id}`}
                        className={`block py-1 px-2 rounded-lg transition-colors truncate ${
                          item.level === 3 ? "pl-4 text-[11px]" : "font-medium"
                        } ${
                          activeHeading === item.id
                            ? "font-semibold text-[var(--text-primary)]"
                            : "text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                        }`}
                      >
                        {item.title}
                      </a>
                    ))}
                  </nav>
                ) : (
                  <div className="text-xs text-zinc-500 italic">
                    No headings on this page.
                  </div>
                )}
              </div>

              {/* Quick Links Card */}
              <div className="space-y-2 border border-[var(--line)] p-5 text-xs text-[var(--text-muted)]">
                <div className="font-semibold text-zinc-900 dark:text-zinc-200">
                  Beejs Ecosystem
                </div>
                <p className="text-[11px] leading-relaxed">
                  Fast, lightweight, native AI & Agent-grade JS/TS runtime
                  written in Rust & V8.
                </p>
                <div className="pt-2 flex items-center gap-2">
                  <a
                    href="https://github.com/zh30/beejs"
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex items-center gap-1 text-[11px] hover:underline"
                  >
                    GitHub Repository →
                  </a>
                </div>
              </div>
            </div>
          </aside>
        </div>
      </div>
    </div>
  );
}
