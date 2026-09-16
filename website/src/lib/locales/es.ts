import type { TranslationSchema } from "./types";
import { BEEJS_VERSION } from "../version";

export const es: TranslationSchema = {
  nav: {
    home: "Inicio",
    docs: "Manual",
    blog: "Notas de Versión",
    github: "GitHub",
  },
  toggle: {
    label: "Idioma",
    en: "English",
    zh: "简体中文",
    es: "Español",
    fr: "Français",
    hi: "हिन्दी",
  },
  theme: {
    system: "Sistema",
    light: "Claro",
    dark: "Oscuro",
    toggle: "Cambiar Tema (Sistema / Claro / Oscuro)",
  },
  footer: {
    statusLabel: "Estado del Sistema",
    statusValue: "Operativo",
    stage: BEEJS_VERSION,
    contact: "Contacto",
    email: "support@bee.zhanghe.dev",
    rights: "Todos los derechos reservados.",
    builtWith: "Construido con Rust y V8",
    docs: "Documentación",
    blog: "Notas de Versión",
    githubRepo: "Repositorio GitHub",
    copyright: `© ${new Date().getFullYear()} Beejs. Código abierto bajo licencia MIT.`,
  },
  home: {
    heroBadge: BEEJS_VERSION,
    heroBadgeSub: "Wasm 2.0 · bundle/compile · URL/fetch/stream RSI",
    heroBanner:
      "Beejs v1.16.0: Wasm 2.0, bee bundle / compile y hot paths de URL/fetch/ReadableStream.",
    heroBannerLink: "/blog/v1.16.0",
    heroTitlePrefix: "El Runtime de Alto Rendimiento para ",
    heroTitleAccent: "JavaScript y TypeScript",
    heroTitleSuffix: " en Rust y V8",
    heroSubtitle:
      "Diseñado desde primeros principios en Rust y Google V8. Aceleración de tensores bee:ai nativa, pipelines de agentes, snapshot mmap V8 2.0 y 100% de compatibilidad Node.js.",
    ctaPrimary: "Explorar Documentación",
    ctaSecondary: "Duelo de Benchmarks",
    ctaNotes: "Notas de Versión",
    copyBtn: "Copiar",
    copiedBtn: "Copiado",
    latestArticle: {
      badge: "Lanzamiento Oficial",
      title: "Beejs v1.16.0: Wasm 2.0, empaquetado y hot paths web",
      desc: "WebAssembly.Memory de copia cero, bee bundle / compile (Preview), suite 2.0 y RSI de URL / fetch / ReadableStream. Conformance 5.0 sigue en 55/55.",
      readTime: "4 min de lectura",
      date: "2026-09-16",
      link: "/blog/v1.16.0",
      action: "Leer Artículo Completo",
    },
    benchmarksHeader: "Duelo Arquitectónico de Rendimiento",
    benchmarksSub:
      "Cifras reales reproducibles en 100,000 iteraciones. Ingeniería de sistemas en Rust sin sobrecarga superando runtimes en C++ y Zig.",
    benchmarksNote:
      "Probado en Apple Silicon / Linux x86_64 bajo cargas aisladas idénticas. Reproducible mediante scripts/run_benchmarks.sh.",
    benchmarksFilterAll: "Todas las Cargas",
    benchmarksFilterCore: "Ejecución Core",
    benchmarksFilterIo: "Memoria e I/O",
    benchmarksFastest: "⚡ Más Rápido",
    benchmarksParity: "🏆 Paridad",
    benchmarks: [
      {
        id: "require",
        category: "core",
        title: "Resolución de Módulos (require)",
        desc: "100,000 búsquedas de módulos con caché en memoria de dos niveles",
        beeValue: "6.52 ms",
        beeOps: "4,601,226 ops/s",
        bunValue: "16.02 ms",
        bunOps: "1,872,659 ops/s",
        nodeValue: "26.96 ms",
        nodeOps: "1,112,759 ops/s",
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
        desc: "100,000 operaciones de ciclo de vida de buffer alineadas por SIMD",
        beeValue: "2.09 ms",
        beeOps: "47,846 ops/s",
        bunValue: "2.62 ms",
        bunOps: "38,167 ops/s",
        nodeValue: "2.50 ms",
        nodeOps: "40,000 ops/s",
        multiplier: "1.20x vs Node · 1.25x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 79.8,
        nodeBar: 83.6,
      },
      {
        id: "eventemitter",
        category: "core",
        title: "Rendimiento de Disparo EventEmitter",
        desc: "100,000 despachos síncronos de listeners",
        beeValue: "0.62 ms",
        beeOps: "161,290 ops/s",
        bunValue: "0.88 ms",
        bunOps: "113,636 ops/s",
        nodeValue: "0.65 ms",
        nodeOps: "153,846 ops/s",
        multiplier: "Menor latencia de despacho en todos los runtimes",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 70.5,
        nodeBar: 95.4,
      },
      {
        id: "objectalloc",
        category: "core",
        title: "Asignación de Objetos y Memoria",
        desc: "100,000 creaciones y destrucciones ligeras de objetos V8",
        beeValue: "2.25 ms",
        beeOps: "44,444 ops/s",
        bunValue: "2.47 ms",
        bunOps: "40,485 ops/s",
        nodeValue: "2.98 ms",
        nodeOps: "33,557 ops/s",
        multiplier: "1.32x vs Node · 1.10x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 91.1,
        nodeBar: 75.5,
      },
      {
        id: "coldstart",
        category: "core",
        title: "Arranque en Frío de CLI (eval 1+1)",
        desc: "Inicio completo de proceso, creación de isolate y finalización",
        beeValue: "18.00 ms",
        beeOps: "Arranque sub-18ms",
        bunValue: "15.20 ms",
        bunOps: "Arranque sub-16ms",
        nodeValue: "34.83 ms",
        nodeOps: "Arranque sub-35ms",
        multiplier: "1.93x más rápido en frío que Node.js",
        isBeeWinner: false,
        beeBar: 84.4,
        bunBar: 100,
        nodeBar: 43.6,
      },
      {
        id: "timers",
        category: "io",
        title: "Latencia del Event Loop con Timers (1,000)",
        desc: "Registro y disparo por lotes de temporizadores concurrentes",
        beeValue: "2.51 ms",
        beeOps: "14.2x más rápido vs v0.4.2",
        bunValue: "2.20 ms",
        bunOps: "Ultra baja latencia",
        nodeValue: "2.40 ms",
        nodeOps: "Latencia estándar libuv",
        multiplier: "14.2x de aceleración vs Beejs v0.4.2 (35.66 ms)",
        isBeeWinner: false,
        beeBar: 87.6,
        bunBar: 100,
        nodeBar: 91.7,
      },
    ],
    telemetryTitle: "Métricas de Rendimiento",
    telemetrySubtitle:
      "Verificado en benchmarks de producción aislados y repetibles.",
    telemetryNote:
      "Todas las métricas medidas con cargo run --release frente a Node v24 y Bun v1.4.",
    telemetry: [
      {
        label: "Resolución de Módulos",
        value: "4.6M ops/s",
        delta: "4.1x vs Node",
        note: "caché de dos niveles",
      },
      {
        label: "Buffer SIMD",
        value: "2.09 ms",
        delta: "#1 Más rápido",
        note: "100k alloc+fill",
      },
      {
        label: "Arranque en Frío",
        value: "< 18 ms",
        delta: "1.93x vs Node",
        note: "arranque CLI instantáneo",
      },
      {
        label: "Conformidad de Pruebas",
        value: "100%",
        delta: "369/369 Rust",
        note: "45 fixtures de Node",
      },
    ],
    sandboxTitle: "server.ts — Arquitectura HTTP Multi-Worker",
    sandboxTag: "Pool de Workers sin Bloqueos",
    sandboxComment:
      "// Respuesta en streaming nativa de Node.js y Web Standard con Pool de Workers",
    sandboxLog:
      "🚀 Servidor escuchando en http://localhost:3000 (8 workers activos)",
    sandboxBoot: "Tiempo de arranque: < 2ms · Pool de hilos listo",
    featuresTitle: "Arquitectura desde Primeros Principios",
    featuresSubtitle:
      "Diseñado en Rust y V8 para maximizar el throughput, eliminar bloqueos y garantizar seguridad.",
    features: [
      {
        title: "Pool de Hilos Multi-Worker",
        desc: "Redes HTTP de alta concurrencia con despacho por canal sin bloqueos entre hilos, eliminando tormentas de hilos.",
      },
      {
        title: "Caché Dual de Módulos en Memoria",
        desc: "Caché require pre-resuelto de dos niveles que omite stat en disco, logrando 4,601,226 ops/s.",
      },
      {
        title: "TypeScript 6.0 Nativo y TSX",
        desc: "Impulsado por oxc: eliminación instantánea de tipos, decoradores Stage 3, palabra clave using y JSX sin sobrecarga de tsc.",
      },
      {
        title: "Sandbox de Seguridad Fail-Closed",
        desc: "Control granular de capacidades vía --sandbox con aislamiento de directorios, registro JSONL en runtime y listas blancas explícitas.",
      },
      {
        title: "Rueda de Temporizadores de Alta Resolución",
        desc: "Colas de sincronización optimizadas en el event loop logrando 2.51ms en 1,000 temporizadores concurrentes (aceleración 14.2x).",
      },
      {
        title: "Paridad con Node.js y Web Platform",
        desc: "Soporte completo para node:http, node:buffer, node:crypto, fetch, WebSocket, WebCrypto y JIT WebAssembly.",
      },
    ],
    systemsTitle: "Subsistemas del Runtime",
    systemsSubtitle:
      "Arquitectura modular de alto rendimiento construida en Rust.",
    systemsMeta: "Mapa de Arquitectura",
    systemsLabel: "subsistema",
    systems: [
      {
        title: "Core de Runtime e Isolate V8",
        desc: "Enlaces directos de V8 en C++ que brindan velocidad nativa, mínima huella de memoria y soporte WASM JIT.",
      },
      {
        title: "Motor TypeScript oxc",
        desc: "Parser AST ultrarrápido en Rust que elimina tipos y transpila sintaxis TS moderna a ES2022 en tiempo submilimétrico.",
      },
      {
        title: "Concurrencia Multi-Worker",
        desc: "Pool de distribución de tareas sin bloqueos que gestiona miles de conexiones HTTP concurrentes sin penalización de creación de hilos.",
      },
      {
        title: "Capa de Compatibilidad Node.js",
        desc: "Implementaciones exhaustivas de fs, path, crypto, buffer, events, http, process, timers y CommonJS require.",
      },
      {
        title: "Capa de Estándares Web",
        desc: "Implementaciones conformes a W3C de fetch, URL, Streams, Blob, Web Crypto, BroadcastChannel y ServiceWorker.",
      },
      {
        title: "Marco de Pruebas Zero-Config",
        desc: "Ejecutor de pruebas compatible con Jest con aserciones integradas, descubrimiento, ejecución paralela y cobertura.",
      },
    ],
    ctaTitle: "¿Listo para el Rendimiento de Próxima Generación?",
    ctaSubtitle:
      "Instala Beejs v1.16.0 para macOS, Linux y Windows con un solo comando.",
    ctaButton: "Leer Guía de Instalación",
    ctaNotesButton: "Leer Notas de Versión",
  },
  docs: {
    title: "Manual del Runtime",
    subtitle:
      "Documentación para desarrolladores y operadores de Beejs v1.16.0.",
    backToHome: "Volver al Inicio",
    groups: [
      {
        title: "Inicio",
        items: [
          { id: "introduction", label: "Visión General" },
          { id: "installation", label: "Instalación" },
          { id: "quick-start", label: "Inicio Rápido" },
        ],
      },
      {
        title: "Runtime",
        items: [
          { id: "v8-isolate-pool", label: "Núcleo del Runtime" },
          {
            id: "isolate-pool",
            label: "IsolatePool Multi-Tenant (bee:pool)",
            badge: "v1.4",
          },
          { id: "jit-optimization", label: "TypeScript" },
          { id: "ai-engine", label: "Motor de IA (bee:ai)" },
          {
            id: "ai-embeddings",
            label: "Embeddings Nativos y Vectores",
            badge: "v1.3",
          },
          { id: "server-mode", label: "Servidor HTTP y Concurrencia" },
          { id: "memory-management", label: "Buffer SIMD y Memoria" },
        ],
      },
      {
        title: "Ecosistema y Herramientas",
        items: [
          {
            id: "embedded-db",
            label: "Base de Datos y Vectores (bee:db & bee:vector)",
            badge: "DB",
          },
          {
            id: "standard-library",
            label: "Biblioteca Estándar (bee:std)",
            badge: "Std",
          },
          {
            id: "package-manager-dlx",
            label: "Ejecutor de Paquetes (bee x / dlx)",
            badge: "CLI",
          },
          {
            id: "deployment-docker",
            label: "Despliegue y Contenedores (bee deploy)",
            badge: "Deploy",
          },
          {
            id: "ide-extension",
            label: "Extensión VS Code Oficial",
            badge: "IDE",
          },
        ],
      },
      {
        title: "Agent y Sandbox",
        items: [
          {
            id: "agent-sandbox",
            label: "Sandbox y Cuotas de Recursos",
            badge: "Agent",
          },
          {
            id: "agent-replay",
            label: "Motor de Reproducción Determinista (bee:replay)",
            badge: "v1.6",
          },
          {
            id: "model-weights",
            label: "Cargador de Pesos GGUF y SafeTensors (bee:weights)",
            badge: "v1.6",
          },
          {
            id: "capability-security",
            label: "Seguridad Basada en Capacidades (bee:security)",
            badge: "v1.6",
          },
          {
            id: "kv-store",
            label: "KV Persistente y Estado Durable (bee:kv)",
            badge: "v1.7",
          },
          {
            id: "tool-synthesis",
            label: "Autosíntesis de Herramientas y OpenAPI (bee:tools)",
            badge: "v1.7",
          },
          {
            id: "hardened-sandbox",
            label: "Sandbox Reforzado y Registros de Auditoría (bee:sandbox)",
            badge: "v1.7",
          },
          {
            id: "agent-bus",
            label: "Bus de Mensajes y PubSub de Agentes (bee:bus)",
            badge: "v1.8",
          },
          {
            id: "streaming-grammar",
            label: "Salida Estructurada en Streaming (bee:grammar)",
            badge: "v1.8",
          },
          {
            id: "agent-checkpoint",
            label: "Puntos de Control y Rollback (bee:checkpoint)",
            badge: "v1.8",
          },
          {
            id: "mcp-protocol",
            label: "Model Context Protocol 2.0 (bee:mcp)",
            badge: "v1.3",
          },
          {
            id: "virtual-fs-sandbox",
            label: "Sistema de Archivos Virtual en RAM",
            badge: "v1.3",
          },
          {
            id: "ffi-native",
            label: "FFI Nativo C ABI (bee:ffi)",
            badge: "v1.4",
          },
          {
            id: "wasm-interop",
            label: "Puente de Memoria Compartida Wasm 2.0 (bee:wasm)",
            badge: "v1.5",
          },
          {
            id: "slm-inference",
            label: "Edge SLM y Decodificación JSON (bee:ai)",
            badge: "v1.4",
          },
          {
            id: "framework-compat",
            label:
              "Compatibilidad de Frameworks npm (Hono / Express / LangChain)",
            badge: "v1.5",
          },
        ],
      },
      {
        title: "Operaciones",
        items: [
          { id: "cli-usage", label: "Uso de CLI" },
          { id: "wintertc-compliance", label: "Cumplimiento WinterTC" },
          { id: "api-reference", label: "Superficie de API" },
          { id: "modules", label: "Módulos" },
        ],
      },
    ],
    sections: {
      introduction: {
        title: "Visión General",
        subtitle: "Runtime en Rust + V8 para JavaScript y TypeScript.",
        body: [
          "Beejs v1.16.0 es un runtime JavaScript/TypeScript en Rust + V8, un binario: bee. Node Conformance 5.0 es 55/55 fixtures — no es compatibilidad drop-in con Node.",
          "El repositorio también conserva informes de etapas anteriores y módulos con feature-flags. Esos documentos son útiles para entender la evolución del diseño, pero la promesa pública se basa en el build predeterminado de Cargo.",
        ],
        cards: [
          {
            title: "CLI Limpio",
            desc: "La salida por defecto de run y eval evita registros internos de arranque.",
          },
          {
            title: "Build por Defecto",
            desc: "Las comprobaciones de release validan exactamente el conjunto de features instaladas.",
          },
        ],
      },
      installation: {
        title: "Instalación",
        subtitle:
          "Instala un binario precompilado o compila desde el código fuente.",
        body: [
          "Los binarios precompilados están disponibles para macOS x86_64, macOS arm64 y Linux x86_64. Otras plataformas pueden compilar desde el código fuente con Rust.",
        ],
        code: [
          "$ curl -fsSL https://bee.zhanghe.dev/install.sh | sh",
          "$ bee --version",
        ],
      },
      "quick-start": {
        title: "Inicio Rápido",
        subtitle: "Ejecuta tu primer script.",
        body: [
          "Crea un archivo JavaScript o TypeScript y ejecútalo mediante el subcomando run.",
        ],
        code: [
          'console.log("Hello from Beejs");',
          "bee run hello.js",
          'bee eval "1 + 1"',
        ],
      },
      "v8-isolate-pool": {
        title: "Núcleo del Runtime",
        subtitle: "El flujo activo de la CLI ejecuta V8 a través de Rust.",
        body: [
          "La entrada binaria predeterminada es src/main.rs. La ejecución del script se gestiona en src/runtime_minimal.rs, que administra el isolate de V8, la inicialización del contexto y el retorno de resultados.",
        ],
        list: [
          "Ejecuta archivos JavaScript con bee run",
          "Evalúa fragmentos con bee eval",
          "Usa bee repl para una consola interactiva",
        ],
      },
      "jit-optimization": {
        title: "TypeScript",
        subtitle:
          "Los archivos TS y TSX son transpilados por oxc antes de ejecutarse.",
        body: [
          "Al pasar un archivo .ts o .tsx a bee run, la CLI lo procesa mediante oxc (sintaxis TypeScript 6.0, solo transpilación). Los tipos se eliminan. using y los decoradores Stage 3 se reducen a ES2022 para el V8 actual. TSX emite React.createElement clásico.",
        ],
        list: [
          "Soporta archivos .ts, .tsx, .mts, .cts y .jsx",
          "No es tsc --noEmit. Si el JS resultante es válido, se ejecutará",
          "Las importaciones con efectos secundarios se conservan, solo se borra import type",
          "bee run examples/basics/typescript_latest.ts",
        ],
      },
      "memory-management": {
        title: "Compatibilidad",
        subtitle: "APIs seleccionadas de Node.js y Web Platform disponibles.",
        body: [
          "La compilación predeterminada incluye capas de compatibilidad para APIs comunes de Node.js y estándares Web. Consulta los ejemplos y tests para validar casos específicos.",
        ],
        list: [
          "Módulos Node.js: fs, path, crypto, buffer, process, timers y require",
          "Web APIs: fetch, URL, Streams, Blob, Events, Timers y Web Crypto",
        ],
      },
      "server-mode": {
        title: "Modo Servidor",
        subtitle:
          "Stub de verificación de estado, no un servidor de aplicaciones.",
        body: [
          'bee serve inicia un listener tiny_http y responde con un JSON fijo {"ok":true}. No ejecuta scripts de usuario. Para servidores HTTP de aplicación, usa http.createServer y bee run.',
        ],
        code: ["$ bee serve --host localhost --port 3000"],
      },
      "cli-usage": {
        title: "Uso de CLI",
        subtitle: "Comandos principales.",
        list: [
          "bee run <archivo> - ejecuta un archivo JavaScript o TypeScript",
          "bee eval <código> - evalúa un fragmento de código",
          "bee test [archivo] - ejecuta el framework de pruebas integrado",
          "bee bundle <entrada> - genera un bundle optimizado para producción",
          "bee serve - stub de comprobación de salud (JSON fijo)",
          "bee install - instala dependencias desde package.json",
        ],
      },
      "api-reference": {
        title: "Superficie de API",
        subtitle: "Consulta el comportamiento frente al runtime actual.",
        body: [
          "Beejs expone un subconjunto práctico de APIs de Node.js y la plataforma Web. La referencia más confiable es la suite de pruebas y el directorio de ejemplos.",
        ],
        list: [
          "console y timers",
          "CommonJS require",
          "fetch y URL",
          "fs, path, crypto, buffer, process",
        ],
      },
      modules: {
        title: "Módulos",
        subtitle: "Límites de módulos predeterminados.",
        list: [
          "src/runtime_minimal.rs - runtime V8 actual",
          "src/nodejs_core/ - módulos de compatibilidad de Node.js",
          "src/web_api/ - módulos de estándares Web API",
          "src/testing/ - framework de pruebas integrado",
          "src/package_manager.rs - soporte para gestión de paquetes",
        ],
      },
    },
  },
  blog: {
    title: "Notas de Versión",
    subtitle:
      "Novedades del runtime, actualizaciones de arquitectura y alcance de versión.",
    tagLabel: "Tema",
    back: "Volver a Notas",
    operator: "Autor",
    by: "Por ",
    timestamp: "Fecha",
    readTime: "Tiempo de Lectura",
    readMore: "Abrir Nota",
    notFound: "Artículo No Encontrado",
    fallbackNote:
      "Este artículo técnico se encuentra disponible temporalmente en inglés.",
  },
};
