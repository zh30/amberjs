import type { TranslationSchema } from "./types";
import { BEEJS_VERSION } from "../version";

export const fr: TranslationSchema = {
  nav: {
    home: "Accueil",
    docs: "Manuel",
    blog: "Notes de Version",
    github: "GitHub",
  },
  toggle: {
    label: "Langue",
    en: "English",
    zh: "简体中文",
    es: "Español",
    fr: "Français",
    hi: "हिन्दी",
  },
  theme: {
    system: "Système",
    light: "Clair",
    dark: "Sombre",
    toggle: "Changer de thème (Système / Clair / Sombre)",
  },
  footer: {
    statusLabel: "État du Système",
    statusValue: "Opérationnel",
    stage: BEEJS_VERSION,
    contact: "Contact",
    email: "support@bee.zhanghe.dev",
    rights: "Tous droits réservés.",
    builtWith: "Conçu avec Rust & V8",
    docs: "Documentation",
    blog: "Notes de Version",
    githubRepo: "Dépôt GitHub",
    copyright: `© ${new Date().getFullYear()} Beejs. Open-source sous licence MIT.`,
  },
  home: {
    heroBadge: BEEJS_VERSION,
    heroBadgeSub: "Wasm 2.0 · bundle/compile · URL/fetch/stream RSI",
    heroBanner:
      "Beejs v1.16.0 : Wasm 2.0, bee bundle / compile, et hot paths URL/fetch/ReadableStream.",
    heroBannerLink: "/blog/v1.16.0",
    heroTitlePrefix: "Le Runtime Ultra-Performant pour ",
    heroTitleAccent: "JavaScript & TypeScript",
    heroTitleSuffix: " en Rust & V8",
    heroSubtitle:
      "Conçu selon les premiers principes en Rust & Google V8. Accélération de tenseurs bee:ai native, pipelines d’agents, snapshot mmap V8 2.0 et compatibilité Node.js à 100%.",
    ctaPrimary: "Explorer la Documentation",
    ctaSecondary: "Duel de Benchmarks",
    ctaNotes: "Notes de Version",
    copyBtn: "Copier",
    copiedBtn: "Copié",
    latestArticle: {
      badge: "Sortie Officielle",
      title: "Beejs v1.16.0 : Wasm 2.0, packaging et hot paths web",
      desc: "WebAssembly.Memory zéro-copie, bee bundle / compile (Preview), suite 2.0, et RSI URL / fetch / ReadableStream. Conformance 5.0 reste à 55/55.",
      readTime: "4 min de lecture",
      date: "2026-09-16",
      link: "/blog/v1.16.0",
      action: "Lire l’Article Complet",
    },
    benchmarksHeader: "Confrontation Architecturale des Performances",
    benchmarksSub:
      "Chiffres réels et reproductibles sur 100 000 itérations. L’ingénierie système Rust sans surcoût surpasse les moteurs C++ et Zig.",
    benchmarksNote:
      "Testé sur Apple Silicon / Linux x86_64 sous des charges isolées identiques. Reproductible via scripts/run_benchmarks.sh.",
    benchmarksFilterAll: "Toutes les Charges",
    benchmarksFilterCore: "Exécution Core",
    benchmarksFilterIo: "Mémoire & I/O",
    benchmarksFastest: "⚡ Le Plus Rapide",
    benchmarksParity: "🏆 Parité",
    benchmarks: [
      {
        id: "require",
        category: "core",
        title: "Résolution de Modules (require)",
        desc: "100 000 résolutions de modules avec cache en mémoire à deux niveaux",
        beeValue: "6.52 ms",
        beeOps: "4 601 226 ops/s",
        bunValue: "16.02 ms",
        bunOps: "1 872 659 ops/s",
        nodeValue: "26.96 ms",
        nodeOps: "1 112 759 ops/s",
        multiplier: "4.13x vs Node · 2.45x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 40.7,
        nodeBar: 24.2,
      },
      {
        id: "buffer",
        category: "io",
        title: "Alloc + Fill + Slice de Buffer",
        desc: "100 000 opérations de cycle de vie de buffer alignées par SIMD",
        beeValue: "2.09 ms",
        beeOps: "47 846 ops/s",
        bunValue: "2.62 ms",
        bunOps: "38 167 ops/s",
        nodeValue: "2.50 ms",
        nodeOps: "40 000 ops/s",
        multiplier: "1.20x vs Node · 1.25x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 79.8,
        nodeBar: 83.6,
      },
      {
        id: "eventemitter",
        category: "core",
        title: "Débit d’Émission EventEmitter",
        desc: "100 000 déclenchements synchrones de listeners",
        beeValue: "0.62 ms",
        beeOps: "161 290 ops/s",
        bunValue: "0.88 ms",
        bunOps: "113 636 ops/s",
        nodeValue: "0.65 ms",
        nodeOps: "153 846 ops/s",
        multiplier: "Plus faible latence d’émission parmi tous les runtimes",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 70.5,
        nodeBar: 95.4,
      },
      {
        id: "objectalloc",
        category: "core",
        title: "Allocation d’Objets & Mémoire",
        desc: "100 000 créations d’objets légers V8",
        beeValue: "2.25 ms",
        beeOps: "44 444 ops/s",
        bunValue: "2.47 ms",
        bunOps: "40 485 ops/s",
        nodeValue: "2.98 ms",
        nodeOps: "33 557 ops/s",
        multiplier: "1.32x vs Node · 1.10x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 91.1,
        nodeBar: 75.5,
      },
      {
        id: "coldstart",
        category: "core",
        title: "Démarrage à Froid CLI (eval 1+1)",
        desc: "Lancement complet du processus, création de l’isolate et arrêt",
        beeValue: "18.00 ms",
        beeOps: "Démarrage sub-18ms",
        bunValue: "15.20 ms",
        bunOps: "Démarrage sub-16ms",
        nodeValue: "34.83 ms",
        nodeOps: "Démarrage sub-35ms",
        multiplier: "1.93x plus rapide à froid que Node.js",
        isBeeWinner: false,
        beeBar: 84.4,
        bunBar: 100,
        nodeBar: 43.6,
      },
      {
        id: "timers",
        category: "io",
        title: "Latence Event Loop des Timers (1 000)",
        desc: "Enregistrement et déclenchement par lots de temporisateurs",
        beeValue: "2.51 ms",
        beeOps: "14.2x plus rapide vs v0.4.2",
        bunValue: "2.20 ms",
        bunOps: "Latence ultra-faible",
        nodeValue: "2.40 ms",
        nodeOps: "Latence libuv standard",
        multiplier: "Accélération 14.2x vs Beejs v0.4.2 (35.66 ms)",
        isBeeWinner: false,
        beeBar: 87.6,
        bunBar: 100,
        nodeBar: 91.7,
      },
    ],
    telemetryTitle: "Métriques de Performance",
    telemetrySubtitle:
      "Vérifiées sur des benchmarks de production isolés et reproductibles.",
    telemetryNote:
      "Toutes les métriques mesurées avec cargo run --release face à Node v24 et Bun v1.4.",
    telemetry: [
      {
        label: "Résolution de Modules",
        value: "4.6M ops/s",
        delta: "4.1x vs Node",
        note: "cache deux niveaux",
      },
      {
        label: "Buffer SIMD",
        value: "2.09 ms",
        delta: "#1 Plus rapide",
        note: "100k alloc+fill",
      },
      {
        label: "Démarrage à Froid",
        value: "< 18 ms",
        delta: "1.93x vs Node",
        note: "boot CLI instantané",
      },
      {
        label: "Conformité des Tests",
        value: "100%",
        delta: "369/369 Rust",
        note: "45 fixtures Node",
      },
    ],
    sandboxTitle: "server.ts — Architecture HTTP Multi-Workers",
    sandboxTag: "Pool de Workers Sans Verrou",
    sandboxComment:
      "// Réponse en streaming native Node.js et Web Standard avec pool de workers",
    sandboxLog:
      "🚀 Serveur à l’écoute sur http://localhost:3000 (8 workers actifs)",
    sandboxBoot: "Temps de démarrage : < 2ms · Pool de threads opérationnel",
    featuresTitle: "Architecture Fondée sur les Premiers Principes",
    featuresSubtitle:
      "Conçu en Rust & V8 pour maximiser le débit, éliminer les blocages et garantir la sécurité.",
    features: [
      {
        title: "Pool de Threads Multi-Workers",
        desc: "Mise en réseau HTTP haute concurrence avec répartition sans verrou entre threads, évitant les tempêtes de création de threads.",
      },
      {
        title: "Double Cache de Modules en Mémoire",
        desc: "Cache require pré-résolu à deux niveaux contournant les appels stat disque, atteignant 4 601 226 ops/s.",
      },
      {
        title: "TypeScript 6.0 Natif & TSX",
        desc: "Propulsé par oxc : suppression instantanée des types, décorateurs Stage 3, mot-clé using et JSX sans le surcoût de tsc.",
      },
      {
        title: "Bac à Sable de Sécurité Fail-Closed",
        desc: "Contrôle granulaire des capacités via --sandbox avec isolation des répertoires, journalisation JSONL à l’exécution et listes blanches.",
      },
      {
        title: "Roue de Temporisateurs Haute Résolution",
        desc: "Files d’attente d’event loop optimisées atteignant 2,51ms sur 1 000 temporisateurs concurrents (gain de 14.2x).",
      },
      {
        title: "Parité Node.js & Plateforme Web",
        desc: "Prise en charge complète de node:http, node:buffer, node:crypto, fetch, WebSocket, WebCrypto et WebAssembly JIT.",
      },
    ],
    systemsTitle: "Sous-Systèmes du Runtime",
    systemsSubtitle: "Architecture modulaire haute performance conçue en Rust.",
    systemsMeta: "Carte d’Architecture",
    systemsLabel: "sous-système",
    systems: [
      {
        title: "Cœur de Runtime & Isolate V8",
        desc: "Liaisons C++ directes avec V8 offrant une exécution native, une empreinte mémoire minimale et le support WASM JIT.",
      },
      {
        title: "Moteur TypeScript oxc",
        desc: "Parseur AST Rust ultra-rapide retirant les types et transpilant la syntaxe TS moderne vers ES2022 en temps submillimétrique.",
      },
      {
        title: "Concurrence Multi-Workers",
        desc: "Pool de distribution sans verrou gérant des milliers de connexions HTTP sans pénalité de création de thread.",
      },
      {
        title: "Couche de Compatibilité Node.js",
        desc: "Implémentations complètes de fs, path, crypto, buffer, events, http, process, timers et CommonJS require.",
      },
      {
        title: "Couche de Standards Web",
        desc: "Implémentations conformes W3C de fetch, URL, Streams, Blob, Web Crypto, BroadcastChannel et ServiceWorker.",
      },
      {
        title: "Framework de Test Zero-Config",
        desc: "Exécuteur de tests compatible Jest avec assertions intégrées, découverte automatique, exécution parallèle et couverture.",
      },
    ],
    ctaTitle: "Prêt pour les Performances Nouvelle Génération ?",
    ctaSubtitle:
      "Installez Beejs v1.16.0 sur macOS, Linux et Windows en une seule commande.",
    ctaButton: "Lire le Guide d’Installation",
    ctaNotesButton: "Lire les Notes de Version",
  },
  docs: {
    title: "Manuel du Runtime",
    subtitle: "Documentation développeur et opérateur pour Beejs v1.16.0.",
    backToHome: "Retour à l’Accueil",
    groups: [
      {
        title: "Démarrage",
        items: [
          { id: "introduction", label: "Présentation" },
          { id: "installation", label: "Installation" },
          { id: "quick-start", label: "Démarrage Rapide" },
        ],
      },
      {
        title: "Runtime",
        items: [
          { id: "v8-isolate-pool", label: "Cœur du Runtime" },
          {
            id: "isolate-pool",
            label: "IsolatePool Multi-Tenant (bee:pool)",
            badge: "v1.4",
          },
          { id: "jit-optimization", label: "TypeScript" },
          { id: "ai-engine", label: "Moteur IA (bee:ai)" },
          {
            id: "ai-embeddings",
            label: "Embeddings natifs & Vecteurs",
            badge: "v1.3",
          },
          { id: "server-mode", label: "Serveur HTTP & Concurrence" },
          { id: "memory-management", label: "Buffer SIMD et Mémoire" },
        ],
      },
      {
        title: "Écosystème & Outils",
        items: [
          {
            id: "embedded-db",
            label: "Base de données & Vecteurs (bee:db & bee:vector)",
            badge: "DB",
          },
          {
            id: "standard-library",
            label: "Bibliothèque standard (bee:std)",
            badge: "Std",
          },
          {
            id: "package-manager-dlx",
            label: "Exécuteur de paquets (bee x / dlx)",
            badge: "CLI",
          },
          {
            id: "deployment-docker",
            label: "Déploiement & Conteneurs (bee deploy)",
            badge: "Deploy",
          },
          {
            id: "ide-extension",
            label: "Extension VS Code officielle",
            badge: "IDE",
          },
        ],
      },
      {
        title: "Agent & Sécurité",
        items: [
          {
            id: "agent-sandbox",
            label: "Bac à sable & Quotas de ressources",
            badge: "Agent",
          },
          {
            id: "agent-replay",
            label: "Moteur de Rejeu Déterministe (bee:replay)",
            badge: "v1.6",
          },
          {
            id: "model-weights",
            label: "Chargeur de Poids GGUF & SafeTensors (bee:weights)",
            badge: "v1.6",
          },
          {
            id: "capability-security",
            label: "Sécurité Basée sur les Capacités (bee:security)",
            badge: "v1.6",
          },
          {
            id: "kv-store",
            label: "KV Persistant & État Durable (bee:kv)",
            badge: "v1.7",
          },
          {
            id: "tool-synthesis",
            label: "Auto-synthèse d’outils & OpenAPI (bee:tools)",
            badge: "v1.7",
          },
          {
            id: "hardened-sandbox",
            label: "Bac à sable Renforcé & Journaux d’Audit (bee:sandbox)",
            badge: "v1.7",
          },
          {
            id: "agent-bus",
            label: "Bus de Messages & PubSub d’Agents (bee:bus)",
            badge: "v1.8",
          },
          {
            id: "streaming-grammar",
            label: "Flux Structuré & Grammaires (bee:grammar)",
            badge: "v1.8",
          },
          {
            id: "agent-checkpoint",
            label: "Points de Contrôle & Restauration (bee:checkpoint)",
            badge: "v1.8",
          },
          {
            id: "mcp-protocol",
            label: "Model Context Protocol 2.0 (bee:mcp)",
            badge: "v1.3",
          },
          {
            id: "virtual-fs-sandbox",
            label: "Système de fichiers virtuel en RAM",
            badge: "v1.3",
          },
          {
            id: "ffi-native",
            label: "FFI Natif C ABI (bee:ffi)",
            badge: "v1.4",
          },
          {
            id: "wasm-interop",
            label: "Pont Mémoire Partagée Wasm 2.0 (bee:wasm)",
            badge: "v1.5",
          },
          {
            id: "slm-inference",
            label: "Edge SLM & Décodage JSON (bee:ai)",
            badge: "v1.4",
          },
          {
            id: "framework-compat",
            label: "Compatibilité Frameworks npm (Hono / Express / LangChain)",
            badge: "v1.5",
          },
        ],
      },
      {
        title: "Exploitation",
        items: [
          { id: "cli-usage", label: "Utilisation CLI" },
          { id: "wintertc-compliance", label: "Conformité WinterTC" },
          { id: "api-reference", label: "Surface d’API" },
          { id: "modules", label: "Modules" },
        ],
      },
    ],
    sections: {
      introduction: {
        title: "Présentation",
        subtitle: "Runtime Rust + V8 pour JavaScript et TypeScript.",
        body: [
          "Beejs v1.16.0 est un runtime JavaScript/TypeScript en Rust + V8, un binaire : bee. Node Conformance 5.0 = 55/55 fixtures — ce n’est pas une compatibilité Node drop-in.",
          "Le dépôt conserve également les rapports d’étapes historiques et les modules soumis à feature-flags. Ces documents éclairent l’évolution architecturale, mais la promesse publique s’appuie sur le build Cargo par défaut.",
        ],
        cards: [
          {
            title: "CLI Épurée",
            desc: "Les sorties par défaut de run et eval n’émettent aucun log de démarrage parasite.",
          },
          {
            title: "Build Standard",
            desc: "Les validations de release ciblent le jeu exact de fonctionnalités fournies aux utilisateurs.",
          },
        ],
      },
      installation: {
        title: "Installation",
        subtitle:
          "Installez une archive précompilée ou compilez depuis les sources.",
        body: [
          "Les archives précompilées sont disponibles pour macOS x86_64, macOS arm64 et Linux x86_64. Les autres plateformes peuvent compiler depuis les sources Rust.",
        ],
        code: [
          "$ curl -fsSL https://bee.zhanghe.dev/install.sh | sh",
          "$ bee --version",
        ],
      },
      "quick-start": {
        title: "Démarrage Rapide",
        subtitle: "Exécutez votre premier script.",
        body: [
          "Créez un fichier JavaScript ou TypeScript et lancez-le avec la sous-commande run.",
        ],
        code: [
          'console.log("Hello from Beejs");',
          "bee run hello.js",
          'bee eval "1 + 1"',
        ],
      },
      "v8-isolate-pool": {
        title: "Cœur du Runtime",
        subtitle: "Le chemin CLI actif exécute V8 à travers Rust.",
        body: [
          "L’entrée binaire par défaut est src/main.rs. L’exécution du script est orchestrée par src/runtime_minimal.rs, qui gère l’isolate V8, la configuration du contexte et le retour des résultats.",
        ],
        list: [
          "Exécutez des fichiers JavaScript avec bee run",
          "Évaluez des expressions avec bee eval",
          "Lancez une console interactive avec bee repl",
        ],
      },
      "jit-optimization": {
        title: "TypeScript",
        subtitle:
          "Les fichiers TS et TSX sont transpilés par oxc avant exécution.",
        body: [
          "Lorsqu’un fichier .ts ou .tsx est transmis à bee run, la CLI le traite via oxc (syntaxe TypeScript 6.0, transpilation seule). Les types sont effacés. using et les décorateurs Stage 3 sont convertis en ES2022 pour V8. TSX produit du React.createElement classique.",
        ],
        list: [
          "Support des extensions .ts, .tsx, .mts, .cts et .jsx",
          "Ce n’est pas tsc --noEmit. Si le JS généré est valide, le code s’exécute",
          "Les imports à effets de bord sont conservés, seul import type est effacé",
          "bee run examples/basics/typescript_latest.ts",
        ],
      },
      "memory-management": {
        title: "Compatibilité",
        subtitle: "Sélection d’APIs Node.js et Web Platform disponibles.",
        body: [
          "La version par défaut inclut des couches de compatibilité pour les APIs courantes de Node.js et du Web. Référez-vous aux exemples et tests pour vérifier la couverture fine.",
        ],
        list: [
          "Modules Node.js : fs, path, crypto, buffer, process, timers et require",
          "APIs Web : fetch, URL, Streams, Blob, events, timers et Web Crypto",
        ],
      },
      "server-mode": {
        title: "Mode Serveur",
        subtitle: "Stub de contrôle d’état, pas un serveur d’application.",
        body: [
          'bee serve démarre un écouteur tiny_http et retourne un JSON fixe {"ok":true}. Il n’exécute pas de scripts utilisateur. Pour un serveur applicatif, utilisez http.createServer et bee run.',
        ],
        code: ["$ bee serve --host localhost --port 3000"],
      },
      "cli-usage": {
        title: "Utilisation CLI",
        subtitle: "Commandes principales.",
        list: [
          "bee run <fichier> - exécute un fichier JavaScript ou TypeScript",
          "bee eval <code> - évalue un extrait JavaScript",
          "bee test [fichier] - lance le framework de test intégré",
          "bee bundle <entrée> - génère un bundle de production",
          "bee serve - stub de vérification d’état (JSON fixe)",
          "bee install - installe les dépendances depuis package.json",
        ],
      },
      "api-reference": {
        title: "Surface d’API",
        subtitle: "Comportement garanti du runtime actuel.",
        body: [
          "Beejs expose un sous-ensemble pratique des APIs Node.js et Web. Les références les plus sûres restent la suite de tests et le répertoire examples.",
        ],
        list: [
          "console et timers",
          "CommonJS require",
          "fetch et URL",
          "fs, path, crypto, buffer, process",
        ],
      },
      modules: {
        title: "Modules",
        subtitle: "Frontières modulaires par défaut.",
        list: [
          "src/runtime_minimal.rs - runtime V8 actuel",
          "src/nodejs_core/ - modules de compatibilité Node.js",
          "src/web_api/ - modules d’APIs Web conformes",
          "src/testing/ - framework de test",
          "src/package_manager.rs - gestionnaire de paquets",
        ],
      },
    },
  },
  blog: {
    title: "Notes de Version",
    subtitle:
      "Évolutions du runtime, mises à jour architecturales et périmètre des versions.",
    tagLabel: "Thème",
    back: "Retour aux Notes",
    operator: "Auteur",
    by: "Par ",
    timestamp: "Date",
    readTime: "Temps de Lecture",
    readMore: "Ouvrir la Note",
    notFound: "Article Introuvable",
    fallbackNote:
      "Cet article technique est temporairement affiché en anglais.",
  },
};
