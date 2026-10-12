import type { TranslationSchema } from "./types";

export const en: TranslationSchema = {
  nav: {
    home: "Home",
    docs: "Docs",
    scope: "Compatibility",
    blog: "Blog",
    play: "Playground",
    github: "GitHub",
    menu: "Menu",
  },
  playground: {
    title: "Playground",
    run: "Run",
    running: "Running",
    language: "Language",
    note: "Runs in your browser (the same engine as this tab), not the amber binary. Amber cannot ship V8 inside WASM. TypeScript is checked and emitted by Monaco — the VS Code editor.",
    output: "Output",
    empty: "Run to see console output.",
    loading: "Loading editor…",
  },
  toggle: {
    label: "Language",
  },
  theme: {
    system: "System",
    light: "Light",
    dark: "Dark",
    toggle: "Switch theme (System / Light / Dark)",
  },
  levels: {
    stable: {
      label: "Stable",
      desc: "Pinned by a written contract and executable tests in CI. Limits are documented next to the API.",
    },
    preview: {
      label: "Preview",
      desc: "In the default build and useful, but the contract, diagnostics, or edge cases are still being tightened.",
    },
    experimental: {
      label: "Experimental",
      desc: "Code exists, but it is not a product promise until it is re-verified and graduated.",
    },
    nongoal: {
      label: "Not supported",
      desc: "Deliberately out of scope today. Amber will fail closed instead of pretending.",
    },
  },
  footer: {
    tagline:
      "A JavaScript and TypeScript runtime built with Rust and V8. Open source under MIT.",
    product: "Product",
    resources: "Resources",
    community: "Community",
    docs: "Documentation",
    scope: "Compatibility",
    blog: "Blog",
    play: "Playground",
    conformance: "Conformance scorecard",
    githubRepo: "GitHub",
    issues: "Issues",
    releases: "Releases",
    license: "MIT License",
  },
  home: {
    eyebrow: "Open source · Rust + V8 · MIT",
    titleLead: "One binary for ",
    titleAccent: "JavaScript and TypeScript",
    subtitle:
      "Amber runs scripts, tests, servers, and agent tools from a single amber executable — with an opt-in capability sandbox and an honest, contract-tested compatibility surface.",
    ctaPrimary: "Get started",
    ctaSecondary: "See what's supported",
    copyBtn: "Copy",
    copiedBtn: "Copied",
    installTabs: {
      unix: "macOS / Linux",
      windows: "Windows",
      source: "From source",
    },
    stats: {
      contracts: "Stable contracts (G1–G69)",
      conformance: "Node conformance fixtures",
      binary: "binary — run, test, serve, bundle, compile",
      version: "current release",
    },
    showcaseTitle: "Write it. Run it. Ship it.",
    showcaseSubtitle:
      "Every snippet below uses a Stable surface. No config files, no plugins.",
    showcase: {
      typescript: {
        label: "TypeScript",
        caption:
          ".ts and .tsx run directly. oxc strips types; stack traces point at your .ts lines. It does not type-check.",
      },
      test: {
        label: "Test",
        caption:
          "Jest-style describe / test / expect with zero-argument discovery and --watch.",
      },
      serve: {
        label: "Serve",
        caption:
          "Export a fetch handler and amber serve speaks HTTP/1.1 to it. Add --https for rustls TLS.",
      },
      sandbox: {
        label: "Sandbox",
        caption:
          "Deny file, network, env, and process access by default. Allow exact paths and hosts, and keep a JSONL audit trail.",
      },
      tools: {
        label: "Agent tools",
        caption:
          "Export functions, print their schemas, and serve them over MCP stdio or JSON-RPC.",
      },
    },
    commandsTitle: "Everything is a subcommand",
    commandsSubtitle:
      "The amber CLI covers the daily loop. Each command below is Stable and has a written contract.",
    commands: {
      run: "Run a JS / TS file, with --watch and --inspect",
      test: "Discover and run Jest-style tests",
      serve: "Serve a fetch handler over HTTP or HTTPS",
      bundle: "Bundle a module graph into one file",
      compile: "Build a single-file executable for this host",
      install: "Install package.json dependencies from npm",
      mcp: "Expose tools as an MCP stdio server",
      repl: "Interactive shell",
    },
    commandsMore: "Full CLI reference",
    compatTitle: "Compatibility, one contract at a time",
    compatSubtitle:
      "Amber is not a Node.js clone. Each module below is Stable for the subset its contract describes — and the contract lists what is missing.",
    compatNode: "Node.js modules",
    compatWeb: "Web platform",
    compatRuntime: "Runtime & agents",
    compatLink: "Browse all 69 contracts",
    conformanceTitle: "56 / 56 conformance fixtures pass",
    conformanceBody:
      "tests/conformance/ runs small Node-style scripts — fs, streams, crypto, child_process, Express, Fastify, Hono and more — on every change. CI fails if any fixture fails or skips. Passing fixtures are a regression gate, not a claim of drop-in Node compatibility.",
    conformanceReproduce: "Reproduce it",
    conformanceLink: "Read the scorecard",
    maturityTitle: "Four honest labels",
    maturitySubtitle:
      "Every capability on this site carries one of these. docs/CURRENT_SCOPE.md in the repo is the source of truth.",
    notTitle: "What Amber does not do (yet)",
    notSubtitle:
      "Saying no clearly is part of the contract. These surfaces are out of scope today.",
    perfTitle: "About performance numbers",
    perfBody:
      "Amber does not publish performance claims right now. Any future number will ship with the commit, exact command, hardware, and correctness checks needed to reproduce it.",
    ctaTitle: "Install amber and run something.",
    ctaSubtitle:
      "Prebuilt archives for macOS, Linux, and Windows, or build from source with Cargo.",
    ctaButton: "Installation guide",
    ctaNotesButton: "Read the blog",
  },
  scope: {
    title: "Compatibility & scope",
    subtitle:
      "Exactly what Amber v1.17.0 supports, what is still in Preview, and what is out of scope. Every Stable row links to its contract.",
    sourceNote:
      "Mirrors docs/CURRENT_SCOPE.md on main. If this page and that file disagree, the file wins.",
    searchPlaceholder: "Filter by module, API, or contract id…",
    filterAll: "All",
    areas: {
      cli: "CLI & tooling",
      runtime: "Runtime",
      node: "Node.js",
      web: "Web platform",
    },
    colSurface: "Surface",
    colId: "Id",
    colLimits: "Known limits",
    contract: "Contract",
    baseScope: "Base",
    qualifierNote:
      "Ids marked tentative or provisional graduated under the same rule and may still be renumbered.",
    noResults: "Nothing matches that filter.",
    stableTitle: "Stable",
    stableSubtitle:
      "Graduated under the Graduation Rule: reachable from the amber binary, described without stage reports, covered by tests, limits documented.",
    previewTitle: "Preview",
    previewSubtitle:
      "Present in the default build. Useful for experiments; not yet a compatibility promise.",
    experimentalTitle: "Experimental",
    experimentalSubtitle:
      "Exists as code or a command surface. Do not rely on it without verifying it yourself.",
    unclassifiedTitle: "Present in code, not yet classified",
    unclassifiedSubtitle:
      "These subcommands, flags and amber:* modules exist in the source tree but are not described by CURRENT_SCOPE. Treat them as Experimental.",
    nongoalTitle: "Not supported",
    nongoalSubtitle:
      "Deliberate non-goals for now. Amber fails closed on these instead of shipping stubs.",
    graduationTitle: "How something becomes Stable",
    graduationRules: [
      "It is reachable through src/main.rs or a documented API in src/lib.rs.",
      "Its behavior is described without relying on historical stage reports.",
      "Current tests or smoke commands cover the documented behavior.",
      "Known limitations are documented next to the capability.",
      "Feature-gated work passes a feature check for that feature.",
    ],
  },
  docs: {
    title: "Docs",
    subtitle: "Manual for Amber v1.17.0.",
    backToHome: "Home",
    searchPlaceholder: "Search docs…",
    onThisPage: "On this page",
    previousPage: "Previous",
    nextPage: "Next",
    noResults: "No matching pages.",
    noHeadings: "No headings on this page.",
    editOnGithub: "Edit this page on GitHub",
    groups: {
      start: "Getting started",
      guides: "Guides",
      compat: "Compatibility",
      agents: "Agents & sandbox",
      experimental: "Experimental",
    },
    statusBanner: {
      experimental:
        "Experimental: this page describes code that is not part of the Stable or Preview scope. Verify behavior yourself before relying on it.",
      preview:
        "Preview: useful and in the default build, but the contract is still being tightened.",
    },
    asideTitle: "Source of truth",
    asideBody:
      "Capability levels follow docs/CURRENT_SCOPE.md. Each Stable API links to its contract file.",
    notFoundTitle: "Page not found",
    notFoundBody:
      "This page was removed or renamed during the v1.17 docs rewrite. Pick a page from the sidebar.",
  },
  blog: {
    title: "Blog",
    subtitle: "Release notes and runtime engineering updates.",
    tagLabel: "Topic",
    back: "All posts",
    operator: "Author",
    by: "By ",
    timestamp: "Date",
    readTime: "Read time",
    readMore: "Read post",
    notFound: "Post not found",
    fallbackNote: "",
    historicalBadge: "Historical post",
    historicalNote:
      "Historical notice: benchmark and performance figures in this post are historical. They have not been reproduced on current main and are not current product claims. For what Amber supports today, see ",
    historicalLink: "Compatibility & scope",
    historicalSuffix: ".",
  },
};
