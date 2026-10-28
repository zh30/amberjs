import type { Lang } from "./locales/types";

export interface LandingCopy {
  skip: string;
  close: string;
  headline: string;
  deck: string;
  sub: string;
  proofBefore: string;
  mit: string;
  proofMid: string;
  scorecard: string;
  proofEnd: string;
  copy: string;
  copied: string;
  copyFailed: string;
  installLabel: string;
  tabs: { unix: string; windows: string; source: string };
  floor: readonly { title: string; body: string }[];
  tagline1: string;
  tagline2a: string;
  tagline2b: string;
  steps: readonly { title: string; a: string; b: string }[];
  notTitle: string;
  not: readonly { title: string; body: string }[];
  faqTitle: string;
  faq: readonly { q: string; a: string }[];
  compatTitle: string;
  compatBefore: string;
  compatAfter: string;
  levelHead: string;
  meaningHead: string;
  levels: readonly { title: string; body: string }[];
  scoreLine: string;
  blogLead: string;
}

const zh: LandingCopy = {
  skip: "跳到内容",
  close: "关闭",
  headline: "一个二进制",
  deck: "跑 JavaScript 和 TypeScript。",
  sub: "给正在评估运行时的工程师。脚本、测试、本地服务和 Agent 工具从这条命令开始。每一项稳定能力都有书面契约，契约同时写明缺什么。",
  proofBefore: "，",
  mit: "MIT 许可",
  proofMid: "。记分卡生成于 2026-09-24，",
  scorecard: "{pass} 项全部通过",
  proofEnd: "。它是回归门禁，不是 Node 替代证明。",
  copy: "复制安装命令",
  copied: "已复制",
  copyFailed: "请选中命令",
  installLabel: "安装方式",
  tabs: { unix: "macOS / Linux", windows: "Windows", source: "源码" },
  floor: [
    { title: "稳定", body: "有书面契约，测试钉在 CI 里，限制写在旁边。" },
    { title: "预览", body: "已在默认构建里。可以试，还不是兼容承诺。" },
    { title: "直接跑 TypeScript", body: "类型被擦掉，堆栈指回 .ts 行号。不做类型检查。" },
    { title: "测试不用另配框架", body: "amber test 发现 describe、test、expect。watch 是整进程重跑。" },
    { title: "实验", body: "代码在。毕业之前，不是产品承诺。" },
    { title: "不支持", body: "明确不做。直接失败，不做桩。" },
    { title: "一个文件对外提供 HTTP", body: "amber serve 跑一个 fetch 处理函数。https 参数用 rustls。" },
    { title: "默认什么都不许碰", body: "沙箱拒绝文件、网络、环境变量和子进程，放行要写明。" },
  ],
  tagline1: "跑这个文件。",
  tagline2a: "先读契约，",
  tagline2b: "再依赖它。",
  steps: [
    { title: "装上", a: "macOS、Linux、Windows，或从源码构建。", b: "卸掉就是删掉这个二进制。" },
    { title: "跑起来", a: "amber run，或进入 repl。", b: "测试写成 describe、test、expect。" },
    { title: "读契约", a: "依赖某个模块之前，", b: "打开它写明的边界。" },
    { title: "导出工具", a: "export-tools 打印 schema 后退出。", b: "不执行工具，也不调用模型。" },
  ],
  notTitle: "不做",
  not: [
    { title: "不是 Node", body: "每个模块只承诺契约里的子集。" },
    { title: "不做类型检查", body: "TypeScript 只擦除类型。" },
    { title: "不是 npm", body: "amber install 不是 npm、yarn 或 pnpm。" },
    { title: "不发性能数字", body: "没有提交、命令、硬件和正确性校验，就不发布。" },
    { title: "parallel 会拒绝", body: "该参数以退出码 2 结束。" },
  ],
  faqTitle: "常见问题",
  faq: [
    { q: "Amber 能换掉 Node 吗？", a: "不能。每个模块只承诺契约里写明的子集。" },
    { q: "稳定是什么意思？", a: "有书面契约，并且有测试钉住。事实来源是 docs/CURRENT_SCOPE.md。" },
    { q: "会做类型检查吗？", a: "不会。只擦除类型。" },
    { q: "有性能数字吗？", a: "目前不发布。仓库里旧文章的性能标题不是当前承诺。" },
    { q: "Windows 能装吗？", a: "能。用 install.ps1，发布页也有 zip。" },
    { q: "amber install 是 npm 吗？", a: "不是。它安装 package.json 里的依赖。add、remove、prune、upgrade、init、x 仍是实验。" },
    { q: "沙箱默认放开吗？", a: "不放开。文件、网络、环境变量和子进程都要写明才能用，并留下 JSONL 审计。" },
    { q: "游乐场能跑代码吗？", a: "能，在这个浏览器标签里跑，不是 amber 二进制。本机用 amber run。" },
  ],
  compatTitle: "兼容性",
  compatBefore: "级别以 ",
  compatAfter: " 为准。记分卡不是替代证明。",
  levelHead: "级别",
  meaningHead: "含义",
  levels: [
    { title: "稳定", body: "有书面契约，测试钉在 CI 里，限制写在旁边。" },
    { title: "预览", body: "已在默认构建里。可以试，还不是兼容承诺。" },
    { title: "实验", body: "代码在。毕业之前，不是产品承诺。" },
    { title: "不支持", body: "明确不做。直接失败，不做桩。" },
  ],
  scoreLine: "记分卡 2026-09-24",
  blogLead: "下面是仓库里的标题。性能和 AI-Native 的说法不是当前承诺。",
};

const en: LandingCopy = {
  skip: "Skip to content",
  close: "Close",
  headline: "One binary",
  deck: "Runs JavaScript and TypeScript.",
  sub: "For engineers evaluating a runtime. Scripts, tests, a local server, and agent tools start from this command. Every stable capability has a written contract, and the contract also says what is missing.",
  proofBefore: ", ",
  mit: "MIT license",
  proofMid: ". Scorecard generated 2026-09-24, ",
  scorecard: "{pass} of {total} passed",
  proofEnd: ". It is a regression gate, not proof that Amber replaces Node.",
  copy: "Copy install command",
  copied: "Copied",
  copyFailed: "Select the command",
  installLabel: "Install method",
  tabs: { unix: "macOS / Linux", windows: "Windows", source: "Source" },
  floor: [
    { title: "Stable", body: "A written contract, tests pinned in CI, limits written beside the API." },
    { title: "Preview", body: "In the default build. You can try it. It is not a compatibility promise." },
    { title: "TypeScript directly", body: "Types are stripped. Stack traces point at the .ts line. No typecheck." },
    { title: "Tests without a framework", body: "amber test finds describe, test, and expect. watch reruns the whole process." },
    { title: "Experimental", body: "The code exists. Until it graduates, it is not a product promise." },
    { title: "Not supported", body: "Explicitly out of scope. It fails outright. No stub." },
    { title: "One file serves HTTP", body: "amber serve runs one fetch handler. The https flag uses rustls." },
    { title: "Nothing is allowed by default", body: "The sandbox denies files, network, environment, and subprocesses until a path or host is named." },
  ],
  tagline1: "Run this file.",
  tagline2a: "Read the contract,",
  tagline2b: "then depend on it.",
  steps: [
    { title: "Install", a: "macOS, Linux, Windows, or a source build.", b: "Uninstalling means deleting the binary." },
    { title: "Run", a: "amber run, or open the repl.", b: "Tests are written with describe, test, and expect." },
    { title: "Read the contract", a: "Before you depend on a module,", b: "open the limits it writes down." },
    { title: "Export tools", a: "export-tools prints the schema and exits.", b: "It does not run the tools or call a model." },
  ],
  notTitle: "Not this",
  not: [
    { title: "Not Node", body: "Each module promises only the subset in its contract." },
    { title: "No typecheck", body: "TypeScript only strips types." },
    { title: "Not npm", body: "amber install is not npm, yarn, or pnpm." },
    { title: "No performance numbers", body: "Nothing is published without a commit, a command, hardware, and a correctness check." },
    { title: "parallel is rejected", body: "That flag exits with status 2." },
  ],
  faqTitle: "Questions",
  faq: [
    { q: "Can Amber replace Node?", a: "No. Each module promises only the subset written in its contract." },
    { q: "What does Stable mean?", a: "A written contract, and tests that pin it. The source of truth is docs/CURRENT_SCOPE.md." },
    { q: "Does it typecheck?", a: "No. It only strips types." },
    { q: "Are there performance numbers?", a: "None are published. Performance titles in older posts are not current promises." },
    { q: "Does Windows install work?", a: "Yes. Use install.ps1. The releases page also has a zip." },
    { q: "Is amber install npm?", a: "No. It installs the dependencies in package.json. add, remove, prune, upgrade, init, and x are still experimental." },
    { q: "Is the sandbox open by default?", a: "No. Files, network, environment variables, and subprocesses must be named before they can be used, and the run leaves a JSONL audit." },
    { q: "Can the playground run code?", a: "Yes, in this browser tab, not in the amber binary. On your machine, use amber run." },
  ],
  compatTitle: "Compatibility",
  compatBefore: "Levels follow ",
  compatAfter: ". The scorecard is not proof that Amber replaces Node.",
  levelHead: "Level",
  meaningHead: "Meaning",
  levels: [
    { title: "Stable", body: "A written contract, tests pinned in CI, limits written beside the API." },
    { title: "Preview", body: "In the default build. You can try it. It is not a compatibility promise." },
    { title: "Experimental", body: "The code exists. Until it graduates, it is not a product promise." },
    { title: "Not supported", body: "Explicitly out of scope. It fails outright. No stub." },
  ],
  scoreLine: "Scorecard 2026-09-24",
  blogLead: "Titles from the repository. Performance claims and “AI-Native” are not current promises.",
};

const es: LandingCopy = {
  skip: "Saltar al contenido",
  close: "Cerrar",
  headline: "Un binario",
  deck: "Ejecuta JavaScript y TypeScript.",
  sub: "Para quien está evaluando un runtime. Scripts, pruebas, un servidor local y herramientas de agente empiezan en este comando. Cada capacidad estable tiene un contrato escrito, y el contrato también dice qué falta.",
  proofBefore: ", ",
  mit: "licencia MIT",
  proofMid: ". Tarjeta generada el 2026-09-24, ",
  scorecard: "{pass} de {total} aprobadas",
  proofEnd: ". Es una compuerta de regresión, no una prueba de que Amber reemplaza a Node.",
  copy: "Copiar el comando",
  copied: "Copiado",
  copyFailed: "Selecciona el comando",
  installLabel: "Forma de instalar",
  tabs: { unix: "macOS / Linux", windows: "Windows", source: "Código" },
  floor: [
    { title: "Estable", body: "Contrato escrito, pruebas fijadas en CI, límites al lado de la API." },
    { title: "Vista previa", body: "Está en la compilación por defecto. Se puede probar. No es una promesa de compatibilidad." },
    { title: "TypeScript directo", body: "Los tipos se borran. La pila señala la línea del .ts. No hay comprobación de tipos." },
    { title: "Pruebas sin otro framework", body: "amber test encuentra describe, test y expect. watch vuelve a ejecutar el proceso entero." },
    { title: "Experimental", body: "El código existe. Hasta que se gradúe, no es una promesa del producto." },
    { title: "No soportado", body: "Queda fuera a propósito. Falla directamente. No hay un stub." },
    { title: "Un archivo sirve HTTP", body: "amber serve ejecuta un manejador fetch. El parámetro https usa rustls." },
    { title: "Por defecto no se permite nada", body: "La caja de arena niega archivos, red, entorno y subprocesos hasta que se nombre una ruta o un host." },
  ],
  tagline1: "Ejecuta este archivo.",
  tagline2a: "Lee el contrato,",
  tagline2b: "luego depende de él.",
  steps: [
    { title: "Instalar", a: "macOS, Linux, Windows, o compilar el código.", b: "Desinstalar es borrar el binario." },
    { title: "Ejecutar", a: "amber run, o entrar al repl.", b: "Las pruebas se escriben con describe, test y expect." },
    { title: "Leer el contrato", a: "Antes de depender de un módulo,", b: "abre los límites que deja escritos." },
    { title: "Exportar herramientas", a: "export-tools imprime el schema y sale.", b: "No ejecuta las herramientas ni llama a un modelo." },
  ],
  notTitle: "No es esto",
  not: [
    { title: "No es Node", body: "Cada módulo promete solo el subconjunto de su contrato." },
    { title: "No comprueba tipos", body: "TypeScript solo borra los tipos." },
    { title: "No es npm", body: "amber install no es npm, yarn ni pnpm." },
    { title: "Sin cifras de rendimiento", body: "No se publican sin un commit, un comando, el hardware y una comprobación de corrección." },
    { title: "parallel se rechaza", body: "Ese parámetro termina con el código 2." },
  ],
  faqTitle: "Preguntas",
  faq: [
    { q: "¿Amber puede reemplazar a Node?", a: "No. Cada módulo promete solo el subconjunto escrito en su contrato." },
    { q: "¿Qué significa estable?", a: "Un contrato escrito y pruebas que lo fijan. La fuente es docs/CURRENT_SCOPE.md." },
    { q: "¿Comprueba tipos?", a: "No. Solo borra los tipos." },
    { q: "¿Hay cifras de rendimiento?", a: "No se publican. Los títulos de rendimiento en artículos viejos no son una promesa actual." },
    { q: "¿Se instala en Windows?", a: "Sí. Usa install.ps1. La página de versiones también tiene un zip." },
    { q: "¿amber install es npm?", a: "No. Instala las dependencias de package.json. add, remove, prune, upgrade, init y x siguen en experimental." },
    { q: "¿La caja de arena está abierta?", a: "No. Archivos, red, variables de entorno y subprocesos hay que nombrarlos, y queda una auditoría JSONL." },
    { q: "¿El patio ejecuta código?", a: "Sí, en esta pestaña del navegador, no en el binario amber. En tu máquina, usa amber run." },
  ],
  compatTitle: "Compatibilidad",
  compatBefore: "Los niveles siguen ",
  compatAfter: ". La tarjeta no prueba que Amber reemplace a Node.",
  levelHead: "Nivel",
  meaningHead: "Significado",
  levels: [
    { title: "Estable", body: "Contrato escrito, pruebas fijadas en CI, límites al lado de la API." },
    { title: "Vista previa", body: "Está en la compilación por defecto. Se puede probar. No es una promesa de compatibilidad." },
    { title: "Experimental", body: "El código existe. Hasta que se gradúe, no es una promesa del producto." },
    { title: "No soportado", body: "Queda fuera a propósito. Falla directamente. No hay un stub." },
  ],
  scoreLine: "Tarjeta 2026-09-24",
  blogLead: "Títulos del repositorio. Las cifras de rendimiento y “AI-Native” no son promesas actuales.",
};

const fr: LandingCopy = {
  skip: "Aller au contenu",
  close: "Fermer",
  headline: "Un binaire",
  deck: "Exécute JavaScript et TypeScript.",
  sub: "Pour qui évalue un runtime. Scripts, tests, serveur local et outils d’agent commencent par cette commande. Chaque capacité stable a un contrat écrit, et le contrat dit aussi ce qui manque.",
  proofBefore: ", ",
  mit: "licence MIT",
  proofMid: ". Fiche générée le 2026-09-24, ",
  scorecard: "{pass} sur {total} réussis",
  proofEnd: ". C’est une porte de régression, pas une preuve qu’Amber remplace Node.",
  copy: "Copier la commande",
  copied: "Copié",
  copyFailed: "Sélectionne la commande",
  installLabel: "Installation",
  tabs: { unix: "macOS / Linux", windows: "Windows", source: "Source" },
  floor: [
    { title: "Stable", body: "Contrat écrit, tests fixés dans la CI, limites à côté de l’API." },
    { title: "Aperçu", body: "Dans le build par défaut. On peut l’essayer. Ce n’est pas une promesse de compatibilité." },
    { title: "TypeScript direct", body: "Les types sont effacés. La pile pointe la ligne du .ts. Pas de vérification de types." },
    { title: "Tests sans autre framework", body: "amber test trouve describe, test et expect. watch relance tout le processus." },
    { title: "Expérimental", body: "Le code existe. Tant qu’il n’est pas diplômé, ce n’est pas une promesse du produit." },
    { title: "Non pris en charge", body: "Hors périmètre, explicitement. Échec direct. Pas de bouchon." },
    { title: "Un fichier sert HTTP", body: "amber serve lance un gestionnaire fetch. Le paramètre https utilise rustls." },
    { title: "Rien n’est permis par défaut", body: "Le bac à sable refuse fichiers, réseau, environnement et sous-processus tant qu’un chemin ou un hôte n’est pas nommé." },
  ],
  tagline1: "Lance ce fichier.",
  tagline2a: "Lis le contrat,",
  tagline2b: "puis dépends-en.",
  steps: [
    { title: "Installer", a: "macOS, Linux, Windows, ou compiler les sources.", b: "Désinstaller, c’est supprimer le binaire." },
    { title: "Lancer", a: "amber run, ou ouvrir le repl.", b: "Les tests s’écrivent avec describe, test et expect." },
    { title: "Lire le contrat", a: "Avant de dépendre d’un module,", b: "ouvre les limites qu’il écrit." },
    { title: "Exporter les outils", a: "export-tools imprime le schema et quitte.", b: "Il n’exécute pas les outils et n’appelle pas de modèle." },
  ],
  notTitle: "Pas ça",
  not: [
    { title: "Pas Node", body: "Chaque module ne promet que le sous-ensemble de son contrat." },
    { title: "Pas de vérification de types", body: "TypeScript ne fait qu’effacer les types." },
    { title: "Pas npm", body: "amber install n’est ni npm, ni yarn, ni pnpm." },
    { title: "Pas de chiffres de performance", body: "Rien n’est publié sans un commit, une commande, le matériel et un contrôle de justesse." },
    { title: "parallel est refusé", body: "Ce paramètre se termine avec le code 2." },
  ],
  faqTitle: "Questions",
  faq: [
    { q: "Amber peut-il remplacer Node ?", a: "Non. Chaque module ne promet que le sous-ensemble écrit dans son contrat." },
    { q: "Que veut dire stable ?", a: "Un contrat écrit, et des tests qui le fixent. La source est docs/CURRENT_SCOPE.md." },
    { q: "Vérifie-t-il les types ?", a: "Non. Il efface seulement les types." },
    { q: "Y a-t-il des chiffres de performance ?", a: "Ils ne sont pas publiés. Les titres de performance des anciens articles ne sont pas une promesse actuelle." },
    { q: "L’installation Windows fonctionne-t-elle ?", a: "Oui. Utilise install.ps1. La page des versions a aussi un zip." },
    { q: "amber install est-il npm ?", a: "Non. Il installe les dépendances de package.json. add, remove, prune, upgrade, init et x restent expérimentaux." },
    { q: "Le bac à sable est-il ouvert ?", a: "Non. Fichiers, réseau, variables d’environnement et sous-processus doivent être nommés, et l’exécution laisse un audit JSONL." },
    { q: "Le bac à sable du site exécute-t-il du code ?", a: "Oui, dans cet onglet du navigateur, pas dans le binaire amber. Sur ta machine, utilise amber run." },
  ],
  compatTitle: "Compatibilité",
  compatBefore: "Les niveaux suivent ",
  compatAfter: ". La fiche ne prouve pas qu’Amber remplace Node.",
  levelHead: "Niveau",
  meaningHead: "Sens",
  levels: [
    { title: "Stable", body: "Contrat écrit, tests fixés dans la CI, limites à côté de l’API." },
    { title: "Aperçu", body: "Dans le build par défaut. On peut l’essayer. Ce n’est pas une promesse de compatibilité." },
    { title: "Expérimental", body: "Le code existe. Tant qu’il n’est pas diplômé, ce n’est pas une promesse du produit." },
    { title: "Non pris en charge", body: "Hors périmètre, explicitement. Échec direct. Pas de bouchon." },
  ],
  scoreLine: "Fiche 2026-09-24",
  blogLead: "Titres du dépôt. Les chiffres de performance et « AI-Native » ne sont pas des promesses actuelles.",
};

const hi: LandingCopy = {
  skip: "सामग्री पर जाएँ",
  close: "बंद करें",
  headline: "एक बाइनरी",
  deck: "JavaScript और TypeScript चलाती है।",
  sub: "रनटाइम जाँच रहे इंजीनियरों के लिए। स्क्रिप्ट, परीक्षण, स्थानीय सर्वर और एजेंट उपकरण इसी कमांड से शुरू होते हैं। हर स्थिर क्षमता का लिखित अनुबंध है, और अनुबंध यह भी लिखता है कि क्या नहीं है।",
  proofBefore: ", ",
  mit: "MIT लाइसेंस",
  proofMid: "। स्कोरकार्ड 2026-09-24 को बना, ",
  scorecard: "{pass} में से {total} पास",
  proofEnd: "। यह प्रतिगमन द्वार है, इस बात का प्रमाण नहीं कि Amber Node की जगह लेता है।",
  copy: "इंस्टॉल कमांड कॉपी करें",
  copied: "कॉपी हो गया",
  copyFailed: "कमांड चुनें",
  installLabel: "इंस्टॉल का तरीका",
  tabs: { unix: "macOS / Linux", windows: "Windows", source: "सोर्स" },
  floor: [
    { title: "स्थिर", body: "लिखित अनुबंध, CI में जमी परीक्षण, सीमाएँ API के पास लिखी हुई।" },
    { title: "पूर्वावलोकन", body: "डिफ़ॉल्ट बिल्ड में है। आज़माया जा सकता है। यह संगतता का वादा नहीं।" },
    { title: "सीधा TypeScript", body: "टाइप हटा दिए जाते हैं। स्टैक .ts पंक्ति दिखाता है। टाइप जाँच नहीं होती।" },
    { title: "बिना दूसरे फ्रेमवर्क के परीक्षण", body: "amber test को describe, test और expect मिलते हैं। watch पूरी प्रक्रिया फिर चलाता है।" },
    { title: "प्रयोगात्मक", body: "कोड मौजूद है। स्नातक होने से पहले यह उत्पाद का वादा नहीं।" },
    { title: "समर्थित नहीं", body: "जानबूझकर बाहर। सीधे असफल। कोई स्टब नहीं।" },
    { title: "एक फ़ाइल HTTP देती है", body: "amber serve एक fetch हैंडलर चलाता है। https पैरामीटर rustls इस्तेमाल करता है।" },
    { title: "डिफ़ॉल्ट में कुछ अनुमति नहीं", body: "सैंडबॉक्स फ़ाइल, नेटवर्क, वातावरण और उप-प्रक्रिया तब तक रोकता है जब तक पथ या होस्ट लिखा न जाए।" },
  ],
  tagline1: "इस फ़ाइल को चलाओ।",
  tagline2a: "पहले अनुबंध पढ़ो,",
  tagline2b: "फिर उस पर निर्भर हो।",
  steps: [
    { title: "इंस्टॉल", a: "macOS, Linux, Windows, या सोर्स से बिल्ड।", b: "हटाना इस बाइनरी को मिटाना है।" },
    { title: "चलाओ", a: "amber run, या repl खोलो।", b: "परीक्षण describe, test और expect से लिखे जाते हैं।" },
    { title: "अनुबंध पढ़ो", a: "किसी मॉड्यूल पर निर्भर होने से पहले,", b: "उसकी लिखी सीमाएँ खोलो।" },
    { title: "उपकरण निर्यात", a: "export-tools स्कीमा छापकर बाहर हो जाता है।", b: "न उपकरण चलाता है, न मॉडल बुलाता है।" },
  ],
  notTitle: "यह नहीं",
  not: [
    { title: "Node नहीं", body: "हर मॉड्यूल केवल अपने अनुबंध का उपसमुच्चय वादा करता है।" },
    { title: "टाइप जाँच नहीं", body: "TypeScript केवल टाइप हटाता है।" },
    { title: "npm नहीं", body: "amber install npm, yarn या pnpm नहीं है।" },
    { title: "प्रदर्शन अंक नहीं", body: "कमिट, कमांड, हार्डवेयर और शुद्धता जाँच के बिना अंक प्रकाशित नहीं होते।" },
    { title: "parallel अस्वीकार", body: "यह पैरामीटर निकास कोड 2 पर खत्म होता है।" },
  ],
  faqTitle: "सवाल",
  faq: [
    { q: "क्या Amber Node की जगह ले सकता है?", a: "नहीं। हर मॉड्यूल केवल अनुबंध में लिखे उपसमुच्चय का वादा करता है।" },
    { q: "स्थिर का मतलब क्या है?", a: "लिखित अनुबंध, और उसे जकड़े परीक्षण। स्रोत docs/CURRENT_SCOPE.md है।" },
    { q: "क्या यह टाइप जाँच करता है?", a: "नहीं। केवल टाइप हटाता है।" },
    { q: "क्या प्रदर्शन अंक हैं?", a: "प्रकाशित नहीं। पुराने लेखों के प्रदर्शन शीर्षक मौजूदा वादा नहीं हैं।" },
    { q: "क्या Windows पर इंस्टॉल होता है?", a: "हाँ। install.ps1 इस्तेमाल करें। रिलीज़ पृष्ठ पर zip भी है।" },
    { q: "क्या amber install npm है?", a: "नहीं। यह package.json की निर्भरताएँ इंस्टॉल करता है। add, remove, prune, upgrade, init और x अभी प्रयोगात्मक हैं।" },
    { q: "क्या सैंडबॉक्स डिफ़ॉल्ट में खुला है?", a: "नहीं। फ़ाइल, नेटवर्क, वातावरण चर और उप-प्रक्रिया को नाम देना पड़ता है, और JSONL ऑडिट बचता है।" },
    { q: "क्या खेल का मैदान कोड चलाता है?", a: "हाँ, इस ब्राउज़र टैब में, amber बाइनरी में नहीं। अपनी मशीन पर amber run इस्तेमाल करें।" },
  ],
  compatTitle: "संगतता",
  compatBefore: "स्तर ",
  compatAfter: " का पालन करते हैं। स्कोरकार्ड इस बात का प्रमाण नहीं कि Amber Node की जगह लेता है।",
  levelHead: "स्तर",
  meaningHead: "अर्थ",
  levels: [
    { title: "स्थिर", body: "लिखित अनुबंध, CI में जमी परीक्षण, सीमाएँ API के पास लिखी हुई।" },
    { title: "पूर्वावलोकन", body: "डिफ़ॉल्ट बिल्ड में है। आज़माया जा सकता है। यह संगतता का वादा नहीं।" },
    { title: "प्रयोगात्मक", body: "कोड मौजूद है। स्नातक होने से पहले यह उत्पाद का वादा नहीं।" },
    { title: "समर्थित नहीं", body: "जानबूझकर बाहर। सीधे असफल। कोई स्टब नहीं।" },
  ],
  scoreLine: "स्कोरकार्ड 2026-09-24",
  blogLead: "रिपॉज़िटरी के शीर्षक। प्रदर्शन दावे और “AI-Native” मौजूदा वादा नहीं हैं।",
};

export const landingCopy: Record<Lang, LandingCopy> = { zh, en, es, fr, hi };

export function fillCount(template: string, pass: number, total: number): string {
  return template.replace("{pass}", String(pass)).replace("{total}", String(total));
}
