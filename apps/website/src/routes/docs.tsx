import { Link, Navigate, useParams } from "react-router-dom";
import {
  useState,
  useEffect,
  useMemo,
  isValidElement,
  type ReactNode,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  Copy,
  Check,
  Search,
  X,
  Info,
  Lightbulb,
  AlertCircle,
  AlertTriangle,
  ListTree,
  Terminal,
  Pencil,
  ChevronDown,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useLang } from "../lib/i18n";
import { DOC_NAV, LEGACY_DOC_REDIRECTS } from "../lib/docs-nav";
import { REPO_URL, SCOPE_DOC_URL } from "../lib/scope";
import { LevelBadge } from "../components/LevelBadge";
import type { Level } from "../lib/locales/types";

const docModules = import.meta.glob("../docs/*.md", {
  query: "?raw",
  eager: true,
  import: "default",
}) as Record<string, string>;

type DocData = {
  data: Record<string, string>;
  content: string;
  file: string;
};

function parseFrontmatter(raw: string): {
  data: Record<string, string>;
  content: string;
} {
  const match = raw.match(/^---\s*([\s\S]*?)\s*---\s*([\s\S]*)$/);
  if (!match) return { data: {}, content: raw };

  const data: Record<string, string> = {};
  match[1]
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

  return { data, content: match[2] };
}

function getDocContent(section: string, lang: string): DocData | null {
  const candidates =
    lang === "en" ? [`${section}.md`] : [`${section}.${lang}.md`, `${section}.md`];
  for (const file of candidates) {
    const raw = docModules[`../docs/${file}`];
    if (raw) return { ...parseFrontmatter(raw), file };
  }
  return null;
}

function docLevel(data: Record<string, string>): Level | null {
  const status = data.status;
  return status === "stable" || status === "preview" || status === "experimental"
    ? status
    : null;
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
  const codeClass = isValidElement<{ className?: string }>(children)
    ? children.props.className
    : undefined;
  const language = codeClass?.match(/language-([\w-]+)/)?.[1] ?? "code";

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
          <span>{language}</span>
        </span>
        <button
          onClick={handleCopy}
          aria-label="Copy code"
          className="inline-flex items-center gap-1.5 hover:text-white"
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
      <pre className="m-0! overflow-x-auto rounded-none! bg-transparent! p-5! font-mono text-[13px] leading-relaxed text-[var(--code-fg)]">
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
      <table className="m-0! w-full border-collapse text-left text-xs sm:text-sm">
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
  const [navSection, setNavSection] = useState<string | null>(null);
  const navOpen = navSection === section;

  const docData = getDocContent(section, lang);

  const toc = docData?.content ? extractToc(docData.content) : [];

  useEffect(() => {
    const handleScroll = () => {
      const headings = document.querySelectorAll("h2[id], h3[id]");
      let current = "";
      for (const el of Array.from(headings)) {
        if (el.getBoundingClientRect().top <= 150) current = el.id;
      }
      setActiveHeading(current);
    };

    window.addEventListener("scroll", handleScroll, { passive: true });
    handleScroll();
    return () => window.removeEventListener("scroll", handleScroll);
  }, [section]);

  const groups = useMemo(
    () =>
      DOC_NAV.map((group) => ({
        key: group.group,
        title: manual.groups[group.group],
        items: group.items.map((id) => {
          const doc = getDocContent(id, lang);
          return {
            id,
            label: doc?.data.nav || doc?.data.title || id,
            level: doc ? docLevel(doc.data) : null,
          };
        }),
      })),
    [lang, manual.groups],
  );

  const allItems = useMemo(() => groups.flatMap((g) => g.items), [groups]);

  const filteredGroups = useMemo(() => {
    const q = searchQuery.toLowerCase().trim();
    if (!q) return groups;
    return groups
      .map((g) => ({
        ...g,
        items: g.items.filter(
          (item) =>
            item.label.toLowerCase().includes(q) ||
            item.id.toLowerCase().includes(q),
        ),
      }))
      .filter((g) => g.items.length > 0);
  }, [groups, searchQuery]);

  const redirect = LEGACY_DOC_REDIRECTS[section];
  if (redirect) return <Navigate to={`/docs/${redirect}`} replace />;

  const currentIndex = allItems.findIndex((item) => item.id === section);
  const prevItem = currentIndex > 0 ? allItems[currentIndex - 1] : null;
  const nextItem =
    currentIndex >= 0 && currentIndex < allItems.length - 1
      ? allItems[currentIndex + 1]
      : null;
  const currentGroup = groups.find((g) =>
    g.items.some((item) => item.id === section),
  );

  const title = docData?.data.title || manual.notFoundTitle;
  const subtitle = docData ? docData.data.subtitle || "" : manual.notFoundBody;
  const level = docData ? docLevel(docData.data) : null;
  const banner =
    level === "experimental"
      ? manual.statusBanner.experimental
      : level === "preview"
        ? manual.statusBanner.preview
        : null;

  const sidebar = (
    <>
      <div className="relative mb-6">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400" />
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder={manual.searchPlaceholder}
          aria-label={manual.searchPlaceholder}
          className="w-full border border-[var(--line)] bg-[var(--bg-page)] py-2 pl-8.5 pr-8 text-xs text-[var(--text-primary)] placeholder-[var(--text-muted)]"
        />
        {searchQuery && (
          <button
            onClick={() => setSearchQuery("")}
            aria-label="Clear search"
            className="absolute right-2.5 top-1/2 -translate-y-1/2 p-0.5 text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      <div className="space-y-6 lg:max-h-[calc(100vh-200px)] lg:overflow-y-auto lg:pr-1">
        {filteredGroups.map((group) => (
          <div key={group.key}>
            <h4 className="meta mb-2 uppercase tracking-wider">{group.title}</h4>
            <div className="space-y-0.5">
              {group.items.map((item) => (
                <Link
                  key={item.id}
                  to={`/docs/${item.id}`}
                  aria-current={section === item.id ? "page" : undefined}
                  className={`flex min-h-12 items-center justify-between gap-2 px-2.5 text-sm ${
                    section === item.id
                      ? "bg-[var(--honey)] font-semibold text-[var(--honey-ink)]"
                      : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  <span className="truncate">{item.label}</span>
                  {item.level && item.level !== "stable" && (
                    <span
                      className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                        item.level === "preview" ? "bg-[var(--honey)]" : "bg-sky-500"
                      }`}
                      title={copy.levels[item.level].label}
                    />
                  )}
                </Link>
              ))}
            </div>
          </div>
        ))}

        {filteredGroups.length === 0 && (
          <div className="py-6 text-center text-xs text-[var(--text-muted)]">
            {manual.noResults}
          </div>
        )}
      </div>
    </>
  );

  return (
    <div className="page">
      <div>
        <div className="grid grid-cols-1 gap-8 lg:grid-cols-[260px_1fr] xl:grid-cols-[260px_1fr_220px] xl:gap-10">
          <aside className="h-fit lg:sticky lg:top-24">
            <button
              type="button"
              onClick={() => setNavSection(navOpen ? null : section)}
              aria-expanded={navOpen}
              className="flex w-full items-center justify-between border border-[var(--line)] px-4 py-3 text-sm font-medium lg:hidden"
            >
              <span>
                {manual.title}
                {currentGroup ? ` · ${currentGroup.title}` : ""}
              </span>
              <ChevronDown
                className={`h-4 w-4 ${navOpen ? "rotate-180" : ""}`}
              />
            </button>
            <div
              className={`${navOpen ? "block" : "hidden"} border border-t-0 border-[var(--line)] p-4 lg:block lg:border-0 lg:p-0`}
            >
              {sidebar}
            </div>
          </aside>

          <main key={section} className="min-w-0">
            <div className="mb-8 space-y-4">
              <div className="flex flex-wrap items-center gap-2 text-xs text-[var(--text-muted)]">
                <Link to="/docs" className="hover:text-[var(--text-primary)]">
                  {manual.title}
                </Link>
                {currentGroup && (
                  <>
                    <span className="text-zinc-400 dark:text-zinc-600">/</span>
                    <span>{currentGroup.title}</span>
                  </>
                )}
              </div>

              <div className="flex flex-wrap items-center gap-3">
                <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
                  {title}
                </h1>
                {level && <LevelBadge level={level} />}
              </div>

              {subtitle && (
                <p className="max-w-3xl text-lg leading-relaxed text-[var(--text-secondary)]">
                  {subtitle}
                </p>
              )}

              {banner && (
                <div
                  className="flex gap-3 border border-[var(--on-paper)] p-4 text-sm"
                >
                  <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                  <span>{banner}</span>
                </div>
              )}

              <div className="h-px w-full bg-[var(--line)]" />
            </div>

            {docData ? (
              <div className="prose prose-amber max-w-none dark:prose-invert prose-a:underline prose-a:underline-offset-2 [&_:not(pre)>code]:bg-[var(--mark)] [&_:not(pre)>code]:px-1 [&_:not(pre)>code]:before:content-none [&_:not(pre)>code]:after:content-none">
                <ReactMarkdown
                  remarkPlugins={[remarkGfm]}
                  components={{
                    pre: PreBlock,
                    blockquote: BlockquoteBlock,
                    table: TableBlock,
                    a: ({ href, children }) =>
                      href?.startsWith("/") ? (
                        <Link to={href}>{children}</Link>
                      ) : (
                        <a href={href} target="_blank" rel="noreferrer">
                          {children}
                        </a>
                      ),
                    h2: ({ children }) => {
                      const id = slugify(nodeText(children));
                      return (
                        <h2
                          id={id}
                          className="mb-4 mt-12 scroll-mt-24 border-b border-[var(--line)] pb-2 text-2xl font-bold tracking-tight"
                        >
                          <a href={`#${id}`} className="text-inherit no-underline! hover:underline!">
                            {children}
                          </a>
                        </h2>
                      );
                    },
                    h3: ({ children }) => {
                      const id = slugify(nodeText(children));
                      return (
                        <h3
                          id={id}
                          className="mb-3 mt-8 scroll-mt-24 text-lg font-bold tracking-tight"
                        >
                          <a href={`#${id}`} className="text-inherit no-underline! hover:underline!">
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
              <Link to="/docs" className="btn-ink">
                <ArrowLeft className="h-4 w-4" /> {manual.title}
              </Link>
            )}

            {docData && (
              <a
                href={`${REPO_URL}/blob/main/apps/website/src/docs/${docData.file}`}
                target="_blank"
                rel="noreferrer"
                className="mt-12 inline-flex items-center gap-1.5 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:underline"
              >
                <Pencil className="h-3.5 w-3.5" /> {manual.editOnGithub}
              </a>
            )}

            <div className="mt-8 grid grid-cols-1 gap-4 border-t border-[var(--line)] pt-8 sm:grid-cols-2">
              {prevItem ? (
                <Link
                  to={`/docs/${prevItem.id}`}
                  className="group flex flex-col border border-[var(--line)] p-4 hover:border-[var(--line-strong)]"
                >
                  <span className="meta mb-1 flex items-center gap-1">
                    <ArrowLeft className="h-3 w-3 transition-transform group-hover:-translate-x-1" />
                    {manual.previousPage}
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
                  className="group flex flex-col items-end border border-[var(--line)] p-4 text-right hover:border-[var(--line-strong)]"
                >
                  <span className="meta mb-1 flex items-center gap-1">
                    {manual.nextPage}
                    <ArrowRight className="h-3 w-3 transition-transform group-hover:translate-x-1" />
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

          <aside className="hidden xl:block">
            <div className="sticky top-24 space-y-4">
              <div>
                <div className="meta mb-3 flex items-center gap-2 font-medium">
                  <ListTree className="h-3.5 w-3.5" />
                  <span>{manual.onThisPage}</span>
                </div>

                {toc.length > 0 ? (
                  <nav className="max-h-[calc(100vh-300px)] space-y-0.5 overflow-y-auto border-l border-[var(--line)] text-xs">
                    {toc.map((item) => (
                      <a
                        key={item.id}
                        href={`#${item.id}`}
                        className={`block truncate py-1 ${
                          item.level === 3 ? "pl-6 text-xs" : "pl-3 text-sm"
                        } ${
                          activeHeading === item.id
                            ? "font-semibold text-[var(--on-paper)]"
                            : "text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                        }`}
                      >
                        {item.title}
                      </a>
                    ))}
                  </nav>
                ) : (
                  <div className="text-xs text-[var(--text-muted)]">
                    {manual.noHeadings}
                  </div>
                )}
              </div>

              <div className="space-y-2 border border-[var(--line)] p-4 text-xs text-[var(--text-muted)]">
                <div className="font-semibold text-[var(--text-primary)]">
                  {manual.asideTitle}
                </div>
                <p className="leading-relaxed">{manual.asideBody}</p>
                <div className="flex flex-col gap-1 pt-1">
                  <Link to="/compatibility" className="hover:underline">
                    {copy.nav.scope} →
                  </Link>
                  <a
                    href={SCOPE_DOC_URL}
                    target="_blank"
                    rel="noreferrer"
                    className="font-mono hover:underline"
                  >
                    CURRENT_SCOPE.md →
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
