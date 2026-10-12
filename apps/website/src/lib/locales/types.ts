export type Lang = "en" | "zh" | "es" | "fr" | "hi";

export interface LangOption {
  code: Lang;
  label: string;
  nativeLabel: string;
  flag: string;
}

export type Level = "stable" | "preview" | "experimental" | "nongoal";

export type DocGroupKey =
  | "start"
  | "guides"
  | "compat"
  | "agents"
  | "experimental";

export type ShowcaseId = "typescript" | "test" | "serve" | "sandbox" | "tools";

export type CommandKey =
  | "run"
  | "test"
  | "serve"
  | "bundle"
  | "compile"
  | "install"
  | "mcp"
  | "repl";

export interface TranslationSchema {
  nav: {
    home: string;
    docs: string;
    scope: string;
    blog: string;
    play: string;
    github: string;
    menu: string;
  };
  playground: {
    title: string;
    run: string;
    running: string;
    language: string;
    note: string;
    output: string;
    empty: string;
    loading: string;
  };
  toggle: {
    label: string;
  };
  theme: {
    system: string;
    light: string;
    dark: string;
    toggle: string;
  };
  levels: Record<Level, { label: string; desc: string }>;
  footer: {
    tagline: string;
    product: string;
    resources: string;
    community: string;
    docs: string;
    scope: string;
    blog: string;
    play: string;
    conformance: string;
    githubRepo: string;
    issues: string;
    releases: string;
    license: string;
  };
  home: {
    eyebrow: string;
    titleLead: string;
    titleAccent: string;
    subtitle: string;
    ctaPrimary: string;
    ctaSecondary: string;
    copyBtn: string;
    copiedBtn: string;
    installTabs: {
      unix: string;
      windows: string;
      source: string;
    };
    stats: {
      contracts: string;
      conformance: string;
      binary: string;
      version: string;
    };
    showcaseTitle: string;
    showcaseSubtitle: string;
    showcase: Record<ShowcaseId, { label: string; caption: string }>;
    commandsTitle: string;
    commandsSubtitle: string;
    commands: Record<CommandKey, string>;
    commandsMore: string;
    compatTitle: string;
    compatSubtitle: string;
    compatNode: string;
    compatWeb: string;
    compatRuntime: string;
    compatLink: string;
    conformanceTitle: string;
    conformanceBody: string;
    conformanceReproduce: string;
    conformanceLink: string;
    maturityTitle: string;
    maturitySubtitle: string;
    notTitle: string;
    notSubtitle: string;
    perfTitle: string;
    perfBody: string;
    ctaTitle: string;
    ctaSubtitle: string;
    ctaButton: string;
    ctaNotesButton: string;
  };
  scope: {
    title: string;
    subtitle: string;
    sourceNote: string;
    searchPlaceholder: string;
    filterAll: string;
    areas: Record<"cli" | "runtime" | "node" | "web", string>;
    colSurface: string;
    colId: string;
    colLimits: string;
    contract: string;
    baseScope: string;
    qualifierNote: string;
    noResults: string;
    stableTitle: string;
    stableSubtitle: string;
    previewTitle: string;
    previewSubtitle: string;
    experimentalTitle: string;
    experimentalSubtitle: string;
    unclassifiedTitle: string;
    unclassifiedSubtitle: string;
    nongoalTitle: string;
    nongoalSubtitle: string;
    graduationTitle: string;
    graduationRules: readonly string[];
  };
  docs: {
    title: string;
    subtitle: string;
    backToHome: string;
    searchPlaceholder: string;
    onThisPage: string;
    previousPage: string;
    nextPage: string;
    noResults: string;
    noHeadings: string;
    editOnGithub: string;
    groups: Record<DocGroupKey, string>;
    statusBanner: {
      experimental: string;
      preview: string;
    };
    asideTitle: string;
    asideBody: string;
    notFoundTitle: string;
    notFoundBody: string;
  };
  blog: {
    title: string;
    subtitle: string;
    tagLabel: string;
    back: string;
    operator: string;
    by: string;
    timestamp: string;
    readTime: string;
    readMore: string;
    notFound: string;
    fallbackNote: string;
    historicalBadge: string;
    historicalNote: string;
    historicalLink: string;
    historicalSuffix: string;
  };
}
