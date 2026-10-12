import type { TranslationSchema } from "./types";

export const es: TranslationSchema = {
  nav: {
    home: "Inicio",
    docs: "Docs",
    scope: "Compatibilidad",
    blog: "Blog",
    play: "Playground",
    github: "GitHub",
    menu: "Menú",
  },
  playground: {
    title: "Playground",
    run: "Ejecutar",
    running: "Ejecutando",
    language: "Lenguaje",
    note: "Se ejecuta en el navegador, no en el binario amber. TypeScript lo comprueba Monaco (el editor de VS Code).",
    output: "Salida",
    empty: "Ejecuta para ver la consola.",
    loading: "Cargando editor…",
  },
  toggle: {
    label: "Idioma",
  },
  theme: {
    system: "Sistema",
    light: "Claro",
    dark: "Oscuro",
    toggle: "Cambiar tema (Sistema / Claro / Oscuro)",
  },
  levels: {
    stable: {
      label: "Stable",
      desc: "Respaldado por un contrato escrito y pruebas ejecutables en CI. Los límites están documentados junto a la API.",
    },
    preview: {
      label: "Preview",
      desc: "Incluido en la compilación por defecto y útil, pero el contrato y los casos límite aún se están ajustando.",
    },
    experimental: {
      label: "Experimental",
      desc: "El código existe, pero no es una promesa de producto hasta que se vuelva a verificar y gradúe.",
    },
    nongoal: {
      label: "No soportado",
      desc: "Fuera de alcance a propósito. Amber falla de forma explícita en lugar de fingir.",
    },
  },
  footer: {
    tagline:
      "Un runtime de JavaScript y TypeScript construido con Rust y V8. Código abierto bajo MIT.",
    product: "Producto",
    resources: "Recursos",
    community: "Comunidad",
    docs: "Documentación",
    scope: "Compatibilidad",
    blog: "Blog",
    play: "Playground",
    conformance: "Resultados de conformidad",
    githubRepo: "GitHub",
    issues: "Issues",
    releases: "Versiones",
    license: "Licencia MIT",
  },
  home: {
    eyebrow: "Código abierto · Rust + V8 · MIT",
    titleLead: "Un solo binario para ",
    titleAccent: "JavaScript y TypeScript",
    subtitle:
      "Amber ejecuta scripts, pruebas, servidores y herramientas para agentes desde un único ejecutable amber, con un sandbox de capacidades opcional y una compatibilidad respaldada por contratos y pruebas.",
    ctaPrimary: "Empezar",
    ctaSecondary: "Qué está soportado",
    copyBtn: "Copiar",
    copiedBtn: "Copiado",
    installTabs: {
      unix: "macOS / Linux",
      windows: "Windows",
      source: "Desde el código",
    },
    stats: {
      contracts: "contratos Stable (G1–G69)",
      conformance: "pruebas de conformidad Node",
      binary: "binario: run, test, serve, bundle, compile",
      version: "versión actual",
    },
    showcaseTitle: "Escríbelo. Ejecútalo. Publícalo.",
    showcaseSubtitle:
      "Cada ejemplo usa una superficie Stable. Sin archivos de configuración ni plugins.",
    showcase: {
      typescript: {
        label: "TypeScript",
        caption:
          ".ts y .tsx se ejecutan directamente. oxc elimina los tipos; las trazas apuntan a tus líneas .ts. No verifica tipos.",
      },
      test: {
        label: "Pruebas",
        caption:
          "describe / test / expect al estilo Jest, con descubrimiento automático y --watch.",
      },
      serve: {
        label: "Servidor",
        caption:
          "Exporta un manejador fetch y amber serve le habla HTTP/1.1. Añade --https para TLS con rustls.",
      },
      sandbox: {
        label: "Sandbox",
        caption:
          "Deniega por defecto archivos, red, entorno y procesos. Permite rutas y hosts exactos y guarda un registro de auditoría JSONL.",
      },
      tools: {
        label: "Herramientas",
        caption:
          "Exporta funciones, imprime sus esquemas y sírvelas por MCP stdio o JSON-RPC.",
      },
    },
    commandsTitle: "Todo es un subcomando",
    commandsSubtitle:
      "La CLI amber cubre el ciclo diario. Cada comando es Stable y tiene un contrato escrito.",
    commands: {
      run: "Ejecuta un archivo JS / TS, con --watch e --inspect",
      test: "Descubre y ejecuta pruebas estilo Jest",
      serve: "Sirve un manejador fetch por HTTP o HTTPS",
      bundle: "Empaqueta un grafo de módulos en un archivo",
      compile: "Crea un ejecutable de un solo archivo para este host",
      install: "Instala dependencias de package.json desde npm",
      mcp: "Expone herramientas como servidor MCP stdio",
      repl: "Shell interactivo",
    },
    commandsMore: "Referencia completa de la CLI",
    compatTitle: "Compatibilidad, contrato a contrato",
    compatSubtitle:
      "Amber no es un clon de Node.js. Cada módulo es Stable para el subconjunto que describe su contrato, y el contrato indica lo que falta.",
    compatNode: "Módulos de Node.js",
    compatWeb: "Plataforma web",
    compatRuntime: "Runtime y agentes",
    compatLink: "Ver los 69 contratos",
    conformanceTitle: "56 / 56 pruebas de conformidad superadas",
    conformanceBody:
      "tests/conformance/ ejecuta pequeños scripts estilo Node (fs, streams, crypto, child_process, Express, Fastify, Hono y más) en cada cambio. La CI falla si alguna prueba falla o se omite. Es una barrera contra regresiones, no una promesa de compatibilidad total con Node.",
    conformanceReproduce: "Reprodúcelo",
    conformanceLink: "Ver resultados",
    maturityTitle: "Cuatro etiquetas honestas",
    maturitySubtitle:
      "Cada capacidad del sitio lleva una de ellas. docs/CURRENT_SCOPE.md en el repositorio es la fuente de verdad.",
    notTitle: "Lo que Amber no hace (todavía)",
    notSubtitle:
      "Decir que no con claridad también es parte del contrato. Estas superficies están fuera de alcance hoy.",
    perfTitle: "Sobre cifras de rendimiento",
    perfBody:
      "Amber no publica cifras de rendimiento por ahora. Cualquier cifra futura incluirá el commit, el comando exacto, el hardware y las comprobaciones de corrección necesarias para reproducirla.",
    ctaTitle: "Instala amber y ejecuta algo.",
    ctaSubtitle:
      "Binarios para macOS, Linux y Windows, o compila desde el código con Cargo.",
    ctaButton: "Guía de instalación",
    ctaNotesButton: "Leer el blog",
  },
  scope: {
    title: "Compatibilidad y alcance",
    subtitle:
      "Qué soporta exactamente Amber v1.17.0, qué sigue en Preview y qué está fuera de alcance. Cada fila Stable enlaza a su contrato.",
    sourceNote:
      "Refleja docs/CURRENT_SCOPE.md en main. Si esta página y ese archivo difieren, manda el archivo.",
    searchPlaceholder: "Filtrar por módulo, API o id de contrato…",
    filterAll: "Todo",
    areas: {
      cli: "CLI y herramientas",
      runtime: "Runtime",
      node: "Node.js",
      web: "Plataforma web",
    },
    colSurface: "Superficie",
    colId: "Id",
    colLimits: "Límites conocidos",
    contract: "Contrato",
    baseScope: "Base",
    qualifierNote:
      "Los ids marcados como tentative o provisional se graduaron con la misma regla y aún pueden renumerarse.",
    noResults: "Nada coincide con ese filtro.",
    stableTitle: "Stable",
    stableSubtitle:
      "Graduado según la regla: accesible desde el binario amber, descrito sin informes de etapa, cubierto por pruebas y con límites documentados.",
    previewTitle: "Preview",
    previewSubtitle:
      "Presente en la compilación por defecto. Útil para experimentar; aún no es una promesa de compatibilidad.",
    experimentalTitle: "Experimental",
    experimentalSubtitle:
      "Existe como código o comando. No dependas de ello sin verificarlo tú mismo.",
    unclassifiedTitle: "En el código, sin clasificar",
    unclassifiedSubtitle:
      "Estos subcomandos, flags y módulos amber:* existen en el código, pero CURRENT_SCOPE no los describe. Trátalos como Experimental.",
    nongoalTitle: "No soportado",
    nongoalSubtitle:
      "Fuera de alcance a propósito por ahora. Amber falla de forma explícita en lugar de ofrecer stubs.",
    graduationTitle: "Cómo algo llega a Stable",
    graduationRules: [
      "Es accesible a través de src/main.rs o de una API documentada en src/lib.rs.",
      "Su comportamiento se describe sin depender de informes de etapas anteriores.",
      "Las pruebas actuales cubren el comportamiento documentado.",
      "Los límites conocidos están documentados junto a la capacidad.",
      "El trabajo con features pasa la comprobación de esa feature.",
    ],
  },
  docs: {
    title: "Docs",
    subtitle: "Manual de Amber v1.17.0.",
    backToHome: "Inicio",
    searchPlaceholder: "Buscar en la documentación…",
    onThisPage: "En esta página",
    previousPage: "Anterior",
    nextPage: "Siguiente",
    noResults: "No hay páginas que coincidan.",
    noHeadings: "Esta página no tiene encabezados.",
    editOnGithub: "Editar esta página en GitHub",
    groups: {
      start: "Primeros pasos",
      guides: "Guías",
      compat: "Compatibilidad",
      agents: "Agentes y sandbox",
      experimental: "Experimental",
    },
    statusBanner: {
      experimental:
        "Experimental: esta página describe código que no forma parte del alcance Stable ni Preview. Verifícalo antes de depender de él.",
      preview:
        "Preview: útil y en la compilación por defecto, pero el contrato aún se está ajustando.",
    },
    asideTitle: "Fuente de verdad",
    asideBody:
      "Los niveles siguen docs/CURRENT_SCOPE.md. Cada API Stable enlaza a su contrato.",
    notFoundTitle: "Página no encontrada",
    notFoundBody:
      "Esta página se eliminó o renombró en la reescritura de la documentación v1.17. Elige una página en la barra lateral.",
  },
  blog: {
    title: "Blog",
    subtitle: "Notas de versión y novedades de ingeniería del runtime.",
    tagLabel: "Tema",
    back: "Todas las entradas",
    operator: "Autor",
    by: "Por ",
    timestamp: "Fecha",
    readTime: "Lectura",
    readMore: "Leer entrada",
    notFound: "Entrada no encontrada",
    fallbackNote:
      "Este artículo técnico se encuentra disponible temporalmente en inglés.",
    historicalBadge: "Entrada histórica",
    historicalNote:
      "Aviso histórico: las cifras de rendimiento y benchmarks de esta entrada son históricas. No se han reproducido en el main actual y no son afirmaciones vigentes del producto. Para ver lo que Amber admite hoy, consulta ",
    historicalLink: "Compatibilidad y alcance",
    historicalSuffix: ".",
  },
};
