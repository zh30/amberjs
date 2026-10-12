import type { TranslationSchema } from "./types";

export const hi: TranslationSchema = {
  nav: {
    home: "होम",
    docs: "डॉक्स",
    scope: "संगतता",
    blog: "ब्लॉग",
    play: "प्लेग्राउंड",
    github: "GitHub",
    menu: "मेनू",
  },
  playground: {
    title: "प्लेग्राउंड",
    run: "चलाएँ",
    running: "चल रहा है",
    language: "भाषा",
    note: "ब्राउज़र में चलता है, amber बाइनरी में नहीं। TypeScript जाँच Monaco (VS Code संपादक) करती है।",
    output: "आउटपुट",
    empty: "कंसोल देखने के लिए चलाएँ।",
    loading: "संपादक लोड हो रहा है…",
  },
  toggle: {
    label: "भाषा",
  },
  theme: {
    system: "सिस्टम",
    light: "लाइट",
    dark: "डार्क",
    toggle: "थीम बदलें (सिस्टम / लाइट / डार्क)",
  },
  levels: {
    stable: {
      label: "Stable",
      desc: "लिखित अनुबंध और CI में चलने वाले परीक्षणों से सुनिश्चित। सीमाएँ API के साथ ही लिखी हैं।",
    },
    preview: {
      label: "Preview",
      desc: "डिफ़ॉल्ट बिल्ड में उपलब्ध और उपयोगी, पर अनुबंध और किनारे के मामले अभी कसे जा रहे हैं।",
    },
    experimental: {
      label: "Experimental",
      desc: "कोड मौजूद है, पर दोबारा सत्यापित होकर आगे बढ़ने तक यह उत्पाद का वादा नहीं है।",
    },
    nongoal: {
      label: "समर्थित नहीं",
      desc: "जानबूझकर अभी दायरे से बाहर। Amber दिखावा करने के बजाय साफ़ तौर पर विफल होता है।",
    },
  },
  footer: {
    tagline:
      "Rust और V8 से बना JavaScript और TypeScript रनटाइम। MIT के तहत ओपन सोर्स।",
    product: "उत्पाद",
    resources: "संसाधन",
    community: "समुदाय",
    docs: "दस्तावेज़",
    scope: "संगतता",
    blog: "ब्लॉग",
    play: "प्लेग्राउंड",
    conformance: "कन्फ़ॉर्मेंस स्कोरकार्ड",
    githubRepo: "GitHub",
    issues: "Issues",
    releases: "रिलीज़",
    license: "MIT लाइसेंस",
  },
  home: {
    eyebrow: "ओपन सोर्स · Rust + V8 · MIT",
    titleLead: "एक बाइनरी, ",
    titleAccent: "JavaScript और TypeScript के लिए",
    subtitle:
      "Amber एक ही amber executable से स्क्रिप्ट, टेस्ट, सर्वर और एजेंट टूल चलाता है — वैकल्पिक क्षमता सैंडबॉक्स और अनुबंध-परीक्षित संगतता के साथ।",
    ctaPrimary: "शुरू करें",
    ctaSecondary: "क्या समर्थित है",
    copyBtn: "कॉपी",
    copiedBtn: "कॉपी हुआ",
    installTabs: {
      unix: "macOS / Linux",
      windows: "Windows",
      source: "सोर्स से",
    },
    stats: {
      contracts: "Stable अनुबंध (G1–G69)",
      conformance: "Node कन्फ़ॉर्मेंस टेस्ट",
      binary: "बाइनरी — run, test, serve, bundle, compile",
      version: "वर्तमान रिलीज़",
    },
    showcaseTitle: "लिखें। चलाएँ। शिप करें।",
    showcaseSubtitle:
      "नीचे का हर उदाहरण केवल Stable सतह का उपयोग करता है। न कॉन्फ़िग फ़ाइल, न प्लगइन।",
    showcase: {
      typescript: {
        label: "TypeScript",
        caption:
          ".ts और .tsx सीधे चलते हैं। oxc टाइप हटाता है; स्टैक ट्रेस आपकी .ts पंक्तियों की ओर इशारा करते हैं। टाइप-चेक नहीं होता।",
      },
      test: {
        label: "टेस्ट",
        caption:
          "Jest जैसा describe / test / expect, अपने-आप खोज और --watch के साथ।",
      },
      serve: {
        label: "सर्वर",
        caption:
          "एक fetch हैंडलर export करें और amber serve उससे HTTP/1.1 में बात करता है। rustls TLS के लिए --https जोड़ें।",
      },
      sandbox: {
        label: "सैंडबॉक्स",
        caption:
          "फ़ाइल, नेटवर्क, env और प्रोसेस एक्सेस डिफ़ॉल्ट रूप से बंद। सटीक पथ और होस्ट की अनुमति दें और JSONL ऑडिट लॉग रखें।",
      },
      tools: {
        label: "एजेंट टूल",
        caption:
          "फ़ंक्शन export करें, उनके स्कीमा प्रिंट करें और MCP stdio या JSON-RPC पर परोसें।",
      },
    },
    commandsTitle: "सब कुछ एक सबकमांड है",
    commandsSubtitle:
      "amber CLI रोज़मर्रा का काम संभालता है। नीचे की हर कमांड Stable है और उसका लिखित अनुबंध है।",
    commands: {
      run: "JS / TS फ़ाइल चलाएँ, --watch और --inspect के साथ",
      test: "Jest जैसे टेस्ट खोजें और चलाएँ",
      serve: "HTTP या HTTPS पर fetch हैंडलर परोसें",
      bundle: "मॉड्यूल ग्राफ़ को एक फ़ाइल में बंडल करें",
      compile: "इस होस्ट के लिए एक-फ़ाइल executable बनाएँ",
      install: "npm से package.json निर्भरताएँ इंस्टॉल करें",
      mcp: "टूल को MCP stdio सर्वर के रूप में दिखाएँ",
      repl: "इंटरैक्टिव शेल",
    },
    commandsMore: "पूरा CLI संदर्भ",
    compatTitle: "संगतता, एक-एक अनुबंध से",
    compatSubtitle:
      "Amber Node.js का क्लोन नहीं है। हर मॉड्यूल केवल अपने अनुबंध में बताए उपसमुच्चय के लिए Stable है — और अनुबंध बताता है कि क्या नहीं है।",
    compatNode: "Node.js मॉड्यूल",
    compatWeb: "वेब प्लेटफ़ॉर्म",
    compatRuntime: "रनटाइम और एजेंट",
    compatLink: "सभी 69 अनुबंध देखें",
    conformanceTitle: "56 / 56 कन्फ़ॉर्मेंस टेस्ट पास",
    conformanceBody:
      "tests/conformance/ हर बदलाव पर Node-शैली की छोटी स्क्रिप्ट चलाता है — fs, streams, crypto, child_process, Express, Fastify, Hono आदि। कोई टेस्ट फ़ेल या स्किप हो तो CI फ़ेल होता है। यह रिग्रेशन से बचाव है, Node का पूर्ण विकल्प होने का दावा नहीं।",
    conformanceReproduce: "ख़ुद दोहराएँ",
    conformanceLink: "स्कोरकार्ड देखें",
    maturityTitle: "चार ईमानदार लेबल",
    maturitySubtitle:
      "इस साइट की हर क्षमता पर इनमें से एक लेबल है। रिपॉज़िटरी का docs/CURRENT_SCOPE.md ही अंतिम स्रोत है।",
    notTitle: "Amber (अभी) क्या नहीं करता",
    notSubtitle:
      "साफ़ ‘नहीं’ कहना भी अनुबंध का हिस्सा है। ये सतहें आज दायरे से बाहर हैं।",
    perfTitle: "प्रदर्शन आँकड़ों के बारे में",
    perfBody:
      "Amber अभी प्रदर्शन के दावे प्रकाशित नहीं करता। भविष्य का हर आँकड़ा commit, सटीक कमांड, हार्डवेयर और शुद्धता जाँच के साथ आएगा ताकि उसे दोहराया जा सके।",
    ctaTitle: "amber इंस्टॉल करें और कुछ चलाएँ।",
    ctaSubtitle:
      "macOS, Linux और Windows के लिए प्रीबिल्ट आर्काइव, या Cargo से सोर्स से बनाएँ।",
    ctaButton: "इंस्टॉलेशन गाइड",
    ctaNotesButton: "ब्लॉग पढ़ें",
  },
  scope: {
    title: "संगतता और दायरा",
    subtitle:
      "Amber v1.17.0 ठीक-ठीक क्या समर्थित करता है, क्या अभी Preview में है और क्या दायरे से बाहर है। हर Stable पंक्ति अपने अनुबंध से जुड़ी है।",
    sourceNote:
      "main पर docs/CURRENT_SCOPE.md का प्रतिबिंब। असहमति होने पर वही फ़ाइल मान्य है।",
    searchPlaceholder: "मॉड्यूल, API या अनुबंध id से फ़िल्टर करें…",
    filterAll: "सभी",
    areas: {
      cli: "CLI और टूलिंग",
      runtime: "रनटाइम",
      node: "Node.js",
      web: "वेब प्लेटफ़ॉर्म",
    },
    colSurface: "सतह",
    colId: "Id",
    colLimits: "ज्ञात सीमाएँ",
    contract: "अनुबंध",
    baseScope: "आधार",
    qualifierNote:
      "tentative या provisional चिह्नित id उसी नियम से आगे बढ़े हैं और उनकी संख्या बदल सकती है।",
    noResults: "इस फ़िल्टर से कुछ नहीं मिला।",
    stableTitle: "Stable",
    stableSubtitle:
      "नियम के तहत आगे बढ़ा: amber बाइनरी से पहुँच योग्य, पुरानी रिपोर्टों के बिना वर्णित, परीक्षणों से कवर, सीमाएँ दर्ज।",
    previewTitle: "Preview",
    previewSubtitle:
      "डिफ़ॉल्ट बिल्ड में मौजूद। प्रयोग के लिए उपयोगी; अभी संगतता का वादा नहीं।",
    experimentalTitle: "Experimental",
    experimentalSubtitle:
      "कोड या कमांड के रूप में मौजूद। स्वयं सत्यापित किए बिना भरोसा न करें।",
    unclassifiedTitle: "कोड में मौजूद, अभी वर्गीकृत नहीं",
    unclassifiedSubtitle:
      "ये सबकमांड, फ़्लैग और amber:* मॉड्यूल सोर्स में हैं, पर CURRENT_SCOPE इनका वर्णन नहीं करता। इन्हें Experimental मानें।",
    nongoalTitle: "समर्थित नहीं",
    nongoalSubtitle:
      "अभी जानबूझकर दायरे से बाहर। Amber स्टब देने के बजाय साफ़ तौर पर विफल होता है।",
    graduationTitle: "कोई क्षमता Stable कैसे बनती है",
    graduationRules: [
      "वह src/main.rs या src/lib.rs की दस्तावेज़ित API से पहुँच योग्य हो।",
      "उसका व्यवहार पुरानी चरण रिपोर्टों पर निर्भर हुए बिना वर्णित हो।",
      "वर्तमान परीक्षण दस्तावेज़ित व्यवहार को कवर करें।",
      "ज्ञात सीमाएँ क्षमता के साथ दर्ज हों।",
      "feature-gated काम उस feature की जाँच पास करे।",
    ],
  },
  docs: {
    title: "डॉक्स",
    subtitle: "Amber v1.17.0 का मैनुअल।",
    backToHome: "होम",
    searchPlaceholder: "डॉक्स खोजें…",
    onThisPage: "इस पेज पर",
    previousPage: "पिछला",
    nextPage: "अगला",
    noResults: "कोई मिलता-जुलता पेज नहीं।",
    noHeadings: "इस पेज पर कोई शीर्षक नहीं।",
    editOnGithub: "GitHub पर यह पेज संपादित करें",
    groups: {
      start: "शुरुआत",
      guides: "गाइड",
      compat: "संगतता",
      agents: "एजेंट और सैंडबॉक्स",
      experimental: "प्रायोगिक",
    },
    statusBanner: {
      experimental:
        "Experimental: यह पेज ऐसे कोड का वर्णन करता है जो Stable या Preview दायरे में नहीं है। भरोसा करने से पहले स्वयं जाँचें।",
      preview:
        "Preview: उपयोगी और डिफ़ॉल्ट बिल्ड में, पर अनुबंध अभी कसा जा रहा है।",
    },
    asideTitle: "अंतिम स्रोत",
    asideBody:
      "क्षमता स्तर docs/CURRENT_SCOPE.md के अनुसार हैं। हर Stable API अपने अनुबंध से जुड़ी है।",
    notFoundTitle: "पेज नहीं मिला",
    notFoundBody:
      "v1.17 डॉक्स पुनर्लेखन में यह पेज हटाया या बदला गया। साइडबार से कोई पेज चुनें।",
  },
  blog: {
    title: "ब्लॉग",
    subtitle: "रिलीज़ नोट्स और रनटाइम इंजीनियरिंग अपडेट।",
    tagLabel: "विषय",
    back: "सभी पोस्ट",
    operator: "लेखक",
    by: "द्वारा ",
    timestamp: "दिनांक",
    readTime: "पठन समय",
    readMore: "पोस्ट पढ़ें",
    notFound: "पोस्ट नहीं मिली",
    fallbackNote: "यह तकनीकी लेख वर्तमान में अंग्रेजी में उपलब्ध है।",
  },
};
