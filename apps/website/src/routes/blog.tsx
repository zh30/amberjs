import { ChevronLeft, History } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useLang, type Lang } from "../lib/i18n";
import { landingCopy } from "../lib/landing-copy";

interface Post {
  slug: string;
  title: string;
  excerpt: string;
  date: string;
  author: string;
  readTime: string;
  tag: string;
  content: string;
  isFallback?: boolean;
  historical?: boolean;
}

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

const modules = import.meta.glob("../blog/*.md", {
  query: "?raw",
  eager: true,
  import: "default",
}) as Record<string, string>;

function getPosts(lang: Lang): Post[] {
  try {
    const postsMap = new Map<string, Post>();

    // Pass 1: Look for posts matching the active language
    Object.entries(modules).forEach(([path, rawContent]) => {
      if (typeof rawContent !== "string") return;
      const filename = path.split("/").pop() || "";
      const langSuffix = `.${lang}.md`;
      const isTargetLang =
        lang === "en"
          ? !filename.includes(".zh.") &&
            !filename.includes(".es.") &&
            !filename.includes(".fr.") &&
            !filename.includes(".hi.")
          : filename.endsWith(langSuffix);
      const slug = filename.replace(/\.[a-z]{2}\.md$/, "").replace(/\.md$/, "");

      if (isTargetLang) {
        const { data, content } = parseFrontmatter(rawContent);
        postsMap.set(slug, {
          slug,
          title: data.title || "Untitled",
          excerpt:
            data.excerpt || content.slice(0, 160).replace(/[#*`]/g, "") + "...",
          date: data.date || "Unknown Date",
          author: data.author || (lang === "zh" ? "Amber 团队" : "Amber Team"),
          readTime:
            data.readTime || (lang === "zh" ? "5 分钟阅读" : "5 min read"),
          tag: data.tag || (lang === "zh" ? "日志" : "Blog"),
          content,
          isFallback: false,
          historical: data.historical === "true",
        });
      }
    });

    // Pass 2: Gracefully fallback missing posts from English default
    Object.entries(modules).forEach(([path, rawContent]) => {
      if (typeof rawContent !== "string") return;
      const filename = path.split("/").pop() || "";
      const isEnglishFile =
        !filename.includes(".zh.") &&
        !filename.includes(".es.") &&
        !filename.includes(".fr.") &&
        !filename.includes(".hi.");
      const slug = filename.replace(/\.[a-z]{2}\.md$/, "").replace(/\.md$/, "");

      if (isEnglishFile && !postsMap.has(slug)) {
        const { data, content } = parseFrontmatter(rawContent);
        postsMap.set(slug, {
          slug,
          title: data.title || "Untitled",
          excerpt:
            data.excerpt || content.slice(0, 160).replace(/[#*`]/g, "") + "...",
          date: data.date || "Unknown Date",
          author: data.author || "Amber Team",
          readTime: data.readTime || "5 min read",
          tag: data.tag || "Blog",
          content,
          isFallback: lang !== "en",
          historical: data.historical === "true",
        });
      }
    });

    return Array.from(postsMap.values()).sort(
      (a, b) => new Date(b.date).getTime() - new Date(a.date).getTime(),
    );
  } catch (err) {
    console.error("Failed to process blog posts:", err);
    return [];
  }
}

export default function BlogComponent() {
  const { slug } = useParams();
  const { copy, lang } = useLang();
  const posts = getPosts(lang);

  if (slug) {
    const post = posts.find((p) => p.slug === slug);
    if (!post)
      return (
        <div className="page text-center text-[var(--text-muted)]">
          {copy.blog.notFound}
        </div>
      );
    return <BlogPostView post={post} />;
  }

  return (
    <div className="sheet">
      <div className="lead">
        <h1>{copy.blog.title}</h1>
        <p className="sub">{landingCopy[lang].blogLead}</p>
      </div>
      <div className="install">
        <ul className="posts">
          {posts.map((post) => (
            <li key={post.slug}>
              <Link to={`/blog/${post.slug}`}>{post.title}</Link>
              <span>
                {post.date}
                {post.historical ? ` · ${copy.blog.historicalBadge}` : ""}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

/** The header already renders the frontmatter title, so drop a leading `# H1`. */
function withoutLeadingTitle(content: string): string {
  return content.replace(/^\s*#\s[^\n]*\n/, "");
}

function BlogPostView({ post }: { post: Post }) {
  const { copy } = useLang();

  return (
    <div className="page">
      <div className="max-w-[720px]">
        <Link
          to="/blog"
          className="mb-8 inline-flex items-center text-sm text-[var(--text-muted)] hover:underline"
        >
          <ChevronLeft className="mr-1 h-4 w-4" /> {copy.blog.back}
        </Link>

        {post.isFallback && copy.blog.fallbackNote && (
          <p className="mb-6 border border-[var(--line)] px-4 py-3 text-sm text-[var(--text-secondary)]">
            {copy.blog.fallbackNote}
          </p>
        )}

        <article>
          <p className="meta">
            {post.tag} · {post.date} · {post.readTime}
          </p>
          <h1 className="mt-3 text-3xl font-bold tracking-tight">
            {post.title}
          </h1>
          <p className="mt-3 border-b border-[var(--line)] pb-6 text-sm text-[var(--text-muted)]">
            {copy.blog.by}
            {post.author}
          </p>
          {post.historical && (
            <aside
              role="note"
              className="mt-6 flex gap-3 border border-[var(--on-paper)] px-4 py-3 text-sm text-[var(--text-primary)]"
            >
              <History className="mt-0.5 h-4 w-4 shrink-0" aria-hidden />
              <p className="m-0">
                {copy.blog.historicalNote}
                <Link to="/compatibility" className="font-medium underline">
                  {copy.blog.historicalLink}
                </Link>
                {copy.blog.historicalSuffix}
              </p>
            </aside>
          )}
          <div className="prose prose-amber dark:prose-invert mt-8 max-w-none prose-a:underline [&_:not(pre)>code]:bg-[var(--mark)] [&_:not(pre)>code]:px-1 [&_:not(pre)>code]:before:content-none [&_:not(pre)>code]:after:content-none [&_pre]:border [&_pre]:border-[var(--line)] [&_pre]:bg-[var(--code-bg)] [&_pre]:p-5 [&_pre_code]:text-[var(--code-fg)]">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                table: ({ node: _node, ...props }) => (
                  <div className="max-w-full overflow-x-auto">
                    <table {...props} />
                  </div>
                ),
              }}
            >
              {withoutLeadingTitle(post.content)}
            </ReactMarkdown>
          </div>
        </article>
      </div>
    </div>
  );
}
