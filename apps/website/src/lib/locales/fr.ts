import type { TranslationSchema } from "./types";

export const fr: TranslationSchema = {
  nav: {
    home: "Accueil",
    docs: "Docs",
    scope: "Compatibilité",
    blog: "Blog",
    play: "Playground",
    github: "GitHub",
    menu: "Menu",
  },
  playground: {
    title: "Playground",
    run: "Exécuter",
    running: "En cours",
    language: "Langage",
    note: "S’exécute dans le navigateur, pas dans le binaire amber. TypeScript est vérifié par Monaco (l’éditeur de VS Code).",
    output: "Sortie",
    empty: "Lancez pour voir la console.",
    loading: "Chargement de l’éditeur…",
  },
  toggle: {
    label: "Langue",
  },
  theme: {
    system: "Système",
    light: "Clair",
    dark: "Sombre",
    toggle: "Changer de thème (Système / Clair / Sombre)",
  },
  levels: {
    stable: {
      label: "Stable",
      desc: "Garanti par un contrat écrit et des tests exécutables en CI. Les limites sont documentées à côté de l’API.",
    },
    preview: {
      label: "Preview",
      desc: "Présent dans la build par défaut et utile, mais le contrat et les cas limites sont encore en cours d’affinage.",
    },
    experimental: {
      label: "Experimental",
      desc: "Le code existe, mais ce n’est pas une promesse produit tant qu’il n’est pas revérifié et promu.",
    },
    nongoal: {
      label: "Non pris en charge",
      desc: "Volontairement hors périmètre. Amber échoue explicitement plutôt que de faire semblant.",
    },
  },
  footer: {
    tagline:
      "Un runtime JavaScript et TypeScript construit avec Rust et V8. Open source sous licence MIT.",
    product: "Produit",
    resources: "Ressources",
    community: "Communauté",
    docs: "Documentation",
    scope: "Compatibilité",
    blog: "Blog",
    play: "Playground",
    conformance: "Tableau de conformité",
    githubRepo: "GitHub",
    issues: "Issues",
    releases: "Versions",
    license: "Licence MIT",
  },
  home: {
    eyebrow: "Open source · Rust + V8 · MIT",
    titleLead: "Un seul binaire pour ",
    titleAccent: "JavaScript et TypeScript",
    subtitle:
      "Amber exécute scripts, tests, serveurs et outils d’agents depuis un unique exécutable amber, avec un sandbox de capacités optionnel et une compatibilité garantie contrat par contrat.",
    ctaPrimary: "Commencer",
    ctaSecondary: "Ce qui est pris en charge",
    copyBtn: "Copier",
    copiedBtn: "Copié",
    installTabs: {
      unix: "macOS / Linux",
      windows: "Windows",
      source: "Depuis les sources",
    },
    stats: {
      contracts: "contrats Stable (G1–G69)",
      conformance: "tests de conformité Node",
      binary: "binaire : run, test, serve, bundle, compile",
      version: "version actuelle",
    },
    showcaseTitle: "Écrivez. Exécutez. Livrez.",
    showcaseSubtitle:
      "Chaque exemple utilise une surface Stable. Pas de fichier de configuration, pas de plugin.",
    showcase: {
      typescript: {
        label: "TypeScript",
        caption:
          "Les fichiers .ts et .tsx s’exécutent directement. oxc retire les types ; les traces pointent vers vos lignes .ts. Pas de vérification de types.",
      },
      test: {
        label: "Tests",
        caption:
          "describe / test / expect façon Jest, découverte automatique et --watch.",
      },
      serve: {
        label: "Serveur",
        caption:
          "Exportez un gestionnaire fetch et amber serve lui parle en HTTP/1.1. Ajoutez --https pour TLS via rustls.",
      },
      sandbox: {
        label: "Sandbox",
        caption:
          "Refuse par défaut fichiers, réseau, environnement et processus. Autorisez des chemins et hôtes précis et gardez un journal d’audit JSONL.",
      },
      tools: {
        label: "Outils d’agents",
        caption:
          "Exportez des fonctions, affichez leurs schémas et servez-les via MCP stdio ou JSON-RPC.",
      },
    },
    commandsTitle: "Tout est une sous-commande",
    commandsSubtitle:
      "La CLI amber couvre la boucle quotidienne. Chaque commande ci-dessous est Stable et a un contrat écrit.",
    commands: {
      run: "Exécute un fichier JS / TS, avec --watch et --inspect",
      test: "Découvre et exécute des tests façon Jest",
      serve: "Sert un gestionnaire fetch en HTTP ou HTTPS",
      bundle: "Regroupe un graphe de modules en un fichier",
      compile: "Crée un exécutable mono-fichier pour cet hôte",
      install: "Installe les dépendances de package.json depuis npm",
      mcp: "Expose des outils comme serveur MCP stdio",
      repl: "Shell interactif",
    },
    commandsMore: "Référence complète de la CLI",
    compatTitle: "La compatibilité, contrat par contrat",
    compatSubtitle:
      "Amber n’est pas un clone de Node.js. Chaque module est Stable pour le sous-ensemble décrit par son contrat, qui liste aussi ce qui manque.",
    compatNode: "Modules Node.js",
    compatWeb: "Plateforme web",
    compatRuntime: "Runtime et agents",
    compatLink: "Parcourir les 69 contrats",
    conformanceTitle: "56 / 56 tests de conformité réussis",
    conformanceBody:
      "tests/conformance/ exécute de petits scripts façon Node (fs, streams, crypto, child_process, Express, Fastify, Hono…) à chaque changement. La CI échoue si un test échoue ou est ignoré. C’est un garde-fou contre les régressions, pas une promesse de compatibilité totale avec Node.",
    conformanceReproduce: "Reproduisez-le",
    conformanceLink: "Voir le tableau",
    maturityTitle: "Quatre étiquettes honnêtes",
    maturitySubtitle:
      "Chaque capacité du site porte l’une d’elles. docs/CURRENT_SCOPE.md dans le dépôt fait foi.",
    notTitle: "Ce qu’Amber ne fait pas (encore)",
    notSubtitle:
      "Dire non clairement fait partie du contrat. Ces surfaces sont hors périmètre aujourd’hui.",
    perfTitle: "À propos des chiffres de performance",
    perfBody:
      "Amber ne publie pas de chiffres de performance pour l’instant. Tout chiffre futur sera accompagné du commit, de la commande exacte, du matériel et des vérifications nécessaires pour le reproduire.",
    ctaTitle: "Installez amber et lancez quelque chose.",
    ctaSubtitle:
      "Archives précompilées pour macOS, Linux et Windows, ou compilation depuis les sources avec Cargo.",
    ctaButton: "Guide d’installation",
    ctaNotesButton: "Lire le blog",
  },
  scope: {
    title: "Compatibilité et périmètre",
    subtitle:
      "Ce qu’Amber v1.17.0 prend en charge, ce qui reste en Preview et ce qui est hors périmètre. Chaque ligne Stable renvoie à son contrat.",
    sourceNote:
      "Reflète docs/CURRENT_SCOPE.md sur main. En cas de désaccord, le fichier fait foi.",
    searchPlaceholder: "Filtrer par module, API ou id de contrat…",
    filterAll: "Tout",
    areas: {
      cli: "CLI et outils",
      runtime: "Runtime",
      node: "Node.js",
      web: "Plateforme web",
    },
    colSurface: "Surface",
    colId: "Id",
    colLimits: "Limites connues",
    contract: "Contrat",
    baseScope: "Base",
    qualifierNote:
      "Les ids marqués tentative ou provisional ont été promus selon la même règle et peuvent encore être renumérotés.",
    noResults: "Aucun résultat pour ce filtre.",
    stableTitle: "Stable",
    stableSubtitle:
      "Promu selon la règle : accessible depuis le binaire amber, décrit sans rapports d’étape, couvert par des tests, limites documentées.",
    previewTitle: "Preview",
    previewSubtitle:
      "Présent dans la build par défaut. Utile pour expérimenter ; pas encore une promesse de compatibilité.",
    experimentalTitle: "Experimental",
    experimentalSubtitle:
      "Existe sous forme de code ou de commande. Ne vous y fiez pas sans vérifier vous-même.",
    unclassifiedTitle: "Présent dans le code, non classé",
    unclassifiedSubtitle:
      "Ces sous-commandes, options et modules amber:* existent dans les sources mais ne sont pas décrits par CURRENT_SCOPE. Considérez-les comme Experimental.",
    nongoalTitle: "Non pris en charge",
    nongoalSubtitle:
      "Volontairement hors périmètre pour l’instant. Amber échoue explicitement plutôt que de livrer des stubs.",
    graduationTitle: "Comment une capacité devient Stable",
    graduationRules: [
      "Elle est accessible via src/main.rs ou une API documentée dans src/lib.rs.",
      "Son comportement est décrit sans s’appuyer sur d’anciens rapports d’étape.",
      "Des tests actuels couvrent le comportement documenté.",
      "Les limites connues sont documentées à côté de la capacité.",
      "Le travail sous feature passe la vérification de cette feature.",
    ],
  },
  docs: {
    title: "Docs",
    subtitle: "Manuel d’Amber v1.17.0.",
    backToHome: "Accueil",
    searchPlaceholder: "Rechercher dans la doc…",
    onThisPage: "Sur cette page",
    previousPage: "Précédent",
    nextPage: "Suivant",
    noResults: "Aucune page correspondante.",
    noHeadings: "Aucun titre sur cette page.",
    editOnGithub: "Modifier cette page sur GitHub",
    groups: {
      start: "Premiers pas",
      guides: "Guides",
      compat: "Compatibilité",
      agents: "Agents et sandbox",
      experimental: "Expérimental",
    },
    statusBanner: {
      experimental:
        "Experimental : cette page décrit du code hors du périmètre Stable et Preview. Vérifiez avant de vous y fier.",
      preview:
        "Preview : utile et dans la build par défaut, mais le contrat est encore en cours d’affinage.",
    },
    asideTitle: "Source de vérité",
    asideBody:
      "Les niveaux suivent docs/CURRENT_SCOPE.md. Chaque API Stable renvoie à son contrat.",
    notFoundTitle: "Page introuvable",
    notFoundBody:
      "Cette page a été supprimée ou renommée lors de la refonte de la doc v1.17. Choisissez une page dans la barre latérale.",
  },
  blog: {
    title: "Blog",
    subtitle: "Notes de version et actualités d’ingénierie du runtime.",
    tagLabel: "Thème",
    back: "Tous les articles",
    operator: "Auteur",
    by: "Par ",
    timestamp: "Date",
    readTime: "Lecture",
    readMore: "Lire l’article",
    notFound: "Article introuvable",
    fallbackNote:
      "Cet article technique est temporairement affiché en anglais.",
    historicalBadge: "Article historique",
    historicalNote:
      "Avis historique : les chiffres de performance et de benchmark de cet article sont historiques. Ils n’ont pas été reproduits sur le main actuel et ne constituent pas des engagements produit. Pour savoir ce qu’Amber prend en charge aujourd’hui, consultez ",
    historicalLink: "Compatibilité et périmètre",
    historicalSuffix: ".",
  },
};
