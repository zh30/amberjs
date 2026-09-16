import { motion, AnimatePresence } from "framer-motion";
import { useState } from "react";
import {
  ArrowRight,
  BarChart3,
  Calendar,
  Check,
  CheckCircle2,
  Clock,
  Copy,
  Cpu,
  FileCode2,
  Flame,
  Gauge,
  Layers,
  Lock,
  Package,
  Server,
  Sparkles,
  Terminal,
  TrendingUp,
  Zap,
} from "lucide-react";
import { Link } from "react-router-dom";
import { useLang } from "../lib/i18n";
import { BEEJS_VERSION } from "../lib/version";

const featureIcons = [
  <Server
    key="server"
    className="w-6 h-6 text-amber-600 dark:text-amber-400"
  />,
  <Zap key="zap" className="w-6 h-6 text-amber-600 dark:text-amber-400" />,
  <FileCode2
    key="filecode"
    className="w-6 h-6 text-amber-600 dark:text-amber-400"
  />,
  <Lock key="lock" className="w-6 h-6 text-amber-600 dark:text-amber-400" />,
  <Gauge key="gauge" className="w-6 h-6 text-amber-600 dark:text-amber-400" />,
  <Layers
    key="layers"
    className="w-6 h-6 text-amber-600 dark:text-amber-400"
  />,
];

const subsystemIcons = [
  <Cpu key="v8" className="w-5 h-5 text-amber-600 dark:text-amber-400" />,
  <FileCode2
    key="oxc"
    className="w-5 h-5 text-amber-600 dark:text-amber-400"
  />,
  <Server key="pool" className="w-5 h-5 text-amber-600 dark:text-amber-400" />,
  <Layers key="node" className="w-5 h-5 text-amber-600 dark:text-amber-400" />,
  <Gauge key="web" className="w-5 h-5 text-amber-600 dark:text-amber-400" />,
  <Package key="pkg" className="w-5 h-5 text-amber-600 dark:text-amber-400" />,
];

type SandboxTab = "http" | "ts" | "buffer";

export default function HomeComponent() {
  const { copy } = useLang();
  const home = copy.home;
  const [copied, setCopied] = useState(false);
  const [benchmarkFilter, setBenchmarkFilter] = useState<"all" | "core" | "io">(
    "all",
  );
  const [activeTab, setActiveTab] = useState<SandboxTab>("http");

  const installCommand = "curl -fsSL https://bee.zhanghe.dev/install.sh | sh";

  const handleCopy = () => {
    navigator.clipboard.writeText(installCommand);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const filteredBenchmarks = home.benchmarks.filter((item) => {
    if (benchmarkFilter === "all") return true;
    return item.category === benchmarkFilter;
  });

  return (
    <div className="relative overflow-hidden pt-6 pb-24">
      {/* Background Cyber Matrix Grid & Glows */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none z-0">
        <div className="cyber-grid radial-mask absolute inset-0 opacity-40 h-[1200px]" />
        <div className="absolute top-[-80px] left-1/2 -translate-x-1/2 w-[700px] h-[400px] flex items-center justify-center">
          <div className="ambient-glow w-full h-full bg-amber-500/15 animate-pulse-slow" />
        </div>
        <div className="ambient-glow top-[650px] right-[-100px] w-[550px] h-[550px] bg-cyan-500/10" />
        <div className="ambient-glow top-[1400px] left-[-150px] w-[600px] h-[600px] bg-amber-600/10" />
      </div>

      {/* Hero Section */}
      <section className="max-w-6xl mx-auto px-6 pt-10 pb-16 text-center relative z-10">
        {/* Floating Release Announcement Pill */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4 }}
          className="inline-flex mb-8"
        >
          <Link
            to={home.heroBannerLink}
            className="group flex items-center gap-2.5 px-4 py-1.5 rounded-full glass-card border border-amber-500/30 hover:border-amber-500/60 bg-amber-500/10 hover:bg-amber-500/15 text-xs font-mono transition-all shadow-md shadow-amber-500/10 hover:shadow-amber-500/20"
          >
            <span className="flex h-2 w-2 relative">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-500"></span>
            </span>
            <span className="font-semibold text-zinc-950 dark:text-white group-hover:text-amber-700 dark:group-hover:text-amber-200 transition-colors">
              {home.heroBadge}
            </span>
            <span className="text-zinc-400 dark:text-zinc-600">•</span>
            <span className="text-amber-800 dark:text-amber-300/90 font-semibold group-hover:underline flex items-center gap-1">
              {home.heroBadgeSub}
              <ArrowRight className="w-3.5 h-3.5 transition-transform group-hover:translate-x-0.5" />
            </span>
          </Link>
        </motion.div>

        {/* Hero Headline */}
        <motion.h1
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="text-5xl sm:text-6xl md:text-7xl lg:text-8xl font-black tracking-tight max-w-5xl mx-auto leading-[1.08] font-display text-zinc-950 dark:text-white"
        >
          {home.heroTitlePrefix}
          <span className="gradient-amber drop-shadow-sm">
            {home.heroTitleAccent}
          </span>
          {home.heroTitleSuffix}
        </motion.h1>

        {/* Subtitle */}
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="mt-7 text-lg sm:text-xl md:text-2xl text-zinc-700 dark:text-zinc-300 max-w-3xl mx-auto font-normal leading-relaxed text-balance"
        >
          {home.heroSubtitle}
        </motion.p>

        {/* Action Buttons */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
          className="mt-10 flex flex-wrap items-center justify-center gap-4"
        >
          <Link
            to="/docs"
            className="w-full sm:w-auto px-8 py-4 rounded-full bg-gradient-to-r from-amber-500 to-amber-400 text-zinc-950 font-bold text-sm hover:from-amber-400 hover:to-amber-300 transition-all flex items-center justify-center gap-2.5 shadow-xl shadow-amber-500/25 hover:shadow-amber-500/40 group hover:scale-[1.02] active:scale-[0.98]"
          >
            <span>{home.ctaPrimary}</span>
            <ArrowRight className="w-4 h-4 transition-transform group-hover:translate-x-1" />
          </Link>

          <a
            href="#benchmarks"
            className="w-full sm:w-auto px-8 py-4 rounded-full glass-card hover:bg-zinc-200/80 dark:hover:bg-zinc-800/80 text-zinc-800 dark:text-zinc-200 hover:text-zinc-950 dark:hover:text-white font-semibold text-sm transition-all flex items-center justify-center gap-2 border border-zinc-300/80 dark:border-zinc-700/60 hover:border-amber-500/40"
          >
            <BarChart3 className="w-4 h-4 text-amber-500 dark:text-amber-400" />
            <span>{home.ctaSecondary}</span>
          </a>

          <Link
            to={home.latestArticle.link}
            className="w-full sm:w-auto px-6 py-4 rounded-full glass-card hover:bg-zinc-200/60 dark:hover:bg-zinc-800/60 text-zinc-700 dark:text-zinc-400 hover:text-zinc-950 dark:hover:text-zinc-200 font-medium text-sm transition-all flex items-center justify-center gap-2 border border-zinc-300/80 dark:border-zinc-800 hover:border-zinc-400 dark:hover:border-zinc-700"
          >
            <Sparkles className="w-4 h-4 text-amber-500 dark:text-amber-400" />
            <span>{home.ctaNotes}</span>
          </Link>
        </motion.div>

        {/* Quick Install Bar */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.4 }}
          className="mt-12 max-w-xl mx-auto"
        >
          <div className="glass-panel rounded-2xl p-2.5 flex items-center justify-between gap-3 text-xs font-mono text-zinc-800 dark:text-zinc-200 border-zinc-300/80 dark:border-zinc-800 hover:border-amber-500/40 transition-all shadow-lg dark:shadow-2xl">
            <div className="flex items-center gap-2.5 px-3 overflow-x-auto truncate">
              <Terminal className="w-4 h-4 text-amber-500 dark:text-amber-400 shrink-0" />
              <span className="select-all text-zinc-900 dark:text-zinc-200 font-medium">
                {installCommand}
              </span>
            </div>
            <button
              onClick={handleCopy}
              className="px-4 py-2 rounded-xl bg-zinc-200 hover:bg-amber-500 hover:text-zinc-950 text-zinc-900 dark:bg-zinc-800 dark:hover:bg-amber-500 dark:hover:text-zinc-950 dark:text-zinc-200 transition-all shrink-0 flex items-center gap-1.5 text-xs font-sans font-semibold cursor-pointer shadow-sm"
              title="Copy to clipboard"
            >
              {copied ? (
                <>
                  <Check className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
                  <span className="text-emerald-700 dark:text-emerald-400 font-bold">
                    {home.copiedBtn}
                  </span>
                </>
              ) : (
                <>
                  <Copy className="w-3.5 h-3.5 text-zinc-500 dark:text-zinc-400 group-hover:text-zinc-950" />
                  <span>{home.copyBtn}</span>
                </>
              )}
            </button>
          </div>
        </motion.div>
      </section>

      {/* High-Impact Performance Telemetry Grid */}
      <section className="max-w-6xl mx-auto px-6 pb-20 relative z-10">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {home.telemetry.map((item, idx) => (
            <motion.div
              key={item.label}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.4, delay: idx * 0.08 }}
              className="glass-card rounded-2xl p-6 border-zinc-200/80 dark:border-zinc-800/80 hover:border-amber-500/40 transition-all group"
            >
              <div className="flex items-center justify-between">
                <span className="text-xs text-zinc-600 dark:text-zinc-400 uppercase tracking-wider font-mono font-medium">
                  {item.label}
                </span>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-amber-500/15 text-amber-800 dark:text-amber-300 font-semibold border border-amber-500/30">
                  {item.delta}
                </span>
              </div>
              <div className="text-3xl sm:text-4xl font-black text-zinc-950 dark:text-white mt-3 font-display tracking-tight group-hover:text-amber-600 dark:group-hover:text-amber-300 transition-colors">
                {item.value}
              </div>
              <div className="text-xs text-zinc-600 dark:text-zinc-400 font-mono mt-2 flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" />
                {item.note}
              </div>
            </motion.div>
          ))}
        </div>
      </section>

      {/* Featured Release Article Callout Card */}
      <section className="max-w-6xl mx-auto px-6 pb-24 relative z-10">
        <motion.div
          initial={{ opacity: 0, scale: 0.98 }}
          whileInView={{ opacity: 1, scale: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className="relative overflow-hidden rounded-3xl glass-panel border-amber-500/30 p-8 sm:p-10 glow-amber"
        >
          <div className="ambient-glow -top-24 -right-24 w-80 h-80 bg-amber-500/20 pointer-events-none" />

          <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-6 relative z-10">
            <div className="max-w-2xl">
              <div className="flex flex-wrap items-center gap-2.5 mb-3 text-xs font-mono">
                <span className="px-3 py-1 rounded-full bg-amber-500/15 text-amber-800 dark:text-amber-300 border border-amber-500/30 font-semibold flex items-center gap-1.5">
                  <Flame className="w-3.5 h-3.5 text-amber-600 dark:text-amber-400" />
                  {home.latestArticle.badge}
                </span>
                <span className="text-zinc-600 dark:text-zinc-400 flex items-center gap-1">
                  <Calendar className="w-3.5 h-3.5" />
                  {home.latestArticle.date}
                </span>
                <span className="text-zinc-400 dark:text-zinc-600">•</span>
                <span className="text-zinc-600 dark:text-zinc-400 flex items-center gap-1">
                  <Clock className="w-3.5 h-3.5" />
                  {home.latestArticle.readTime}
                </span>
              </div>

              <h2 className="text-2xl sm:text-3xl font-extrabold text-zinc-950 dark:text-white font-display tracking-tight leading-snug">
                {home.latestArticle.title}
              </h2>

              <p className="mt-2.5 text-sm sm:text-base text-zinc-700 dark:text-zinc-300 leading-relaxed font-normal">
                {home.latestArticle.desc}
              </p>
            </div>

            <Link
              to={home.latestArticle.link}
              className="px-6 py-3.5 rounded-full bg-zinc-950 text-white hover:bg-amber-500 hover:text-zinc-950 dark:bg-white dark:text-zinc-950 dark:hover:bg-amber-400 font-bold text-sm transition-all flex items-center gap-2 shrink-0 shadow-lg group hover:scale-105"
            >
              <span>{home.latestArticle.action}</span>
              <ArrowRight className="w-4 h-4 transition-transform group-hover:translate-x-1" />
            </Link>
          </div>
        </motion.div>
      </section>

      {/* Interactive Benchmark Showdown Section */}
      <section
        id="benchmarks"
        className="max-w-6xl mx-auto px-6 pb-28 relative z-10"
      >
        <div className="text-center max-w-3xl mx-auto mb-12">
          <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full glass-card border-amber-500/30 text-xs font-mono text-amber-800 dark:text-amber-300 font-semibold mb-4">
            <TrendingUp className="w-3.5 h-3.5 text-amber-600 dark:text-amber-400" />
            <span>100,000 Operations / Suite</span>
          </div>

          <h2 className="text-3xl sm:text-4xl md:text-5xl font-extrabold text-zinc-950 dark:text-white font-display tracking-tight">
            {home.benchmarksHeader}
          </h2>

          <p className="mt-4 text-base sm:text-lg text-zinc-700 dark:text-zinc-300 font-normal leading-relaxed">
            {home.benchmarksSub}
          </p>

          <p className="mt-2 text-xs text-zinc-600 dark:text-zinc-400 font-mono">
            {home.benchmarksNote}
          </p>

          {/* Filter Tabs */}
          <div className="mt-8 inline-flex p-1.5 rounded-full glass-panel border-zinc-200/80 dark:border-zinc-800">
            <button
              onClick={() => setBenchmarkFilter("all")}
              className={`px-5 py-2 rounded-full text-xs font-semibold font-mono transition-all cursor-pointer ${
                benchmarkFilter === "all"
                  ? "bg-amber-500 text-zinc-950 shadow-md font-bold"
                  : "text-zinc-700 dark:text-zinc-400 hover:text-zinc-950 dark:hover:text-white"
              }`}
            >
              {home.benchmarksFilterAll}
            </button>
            <button
              onClick={() => setBenchmarkFilter("core")}
              className={`px-5 py-2 rounded-full text-xs font-semibold font-mono transition-all cursor-pointer ${
                benchmarkFilter === "core"
                  ? "bg-amber-500 text-zinc-950 shadow-md font-bold"
                  : "text-zinc-700 dark:text-zinc-400 hover:text-zinc-950 dark:hover:text-white"
              }`}
            >
              {home.benchmarksFilterCore}
            </button>
            <button
              onClick={() => setBenchmarkFilter("io")}
              className={`px-5 py-2 rounded-full text-xs font-semibold font-mono transition-all cursor-pointer ${
                benchmarkFilter === "io"
                  ? "bg-amber-500 text-zinc-950 shadow-md font-bold"
                  : "text-zinc-700 dark:text-zinc-400 hover:text-zinc-950 dark:hover:text-white"
              }`}
            >
              {home.benchmarksFilterIo}
            </button>
          </div>
        </div>

        {/* Benchmark Cards Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <AnimatePresence mode="popLayout">
            {filteredBenchmarks.map((bench) => (
              <motion.div
                key={bench.id}
                layout
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.95 }}
                transition={{ duration: 0.3 }}
                className="glass-card rounded-2xl p-6 sm:p-7 border-zinc-200/80 dark:border-zinc-800/80 hover:border-amber-500/40 transition-all flex flex-col justify-between"
              >
                <div>
                  <div className="flex items-start justify-between gap-4 mb-3">
                    <div>
                      <h3 className="text-lg sm:text-xl font-bold text-zinc-950 dark:text-white font-display">
                        {bench.title}
                      </h3>
                      <p className="text-xs text-zinc-600 dark:text-zinc-400 mt-1">
                        {bench.desc}
                      </p>
                    </div>

                    <span className="shrink-0 px-2.5 py-1 rounded-full bg-amber-500/15 text-amber-800 dark:text-amber-300 border border-amber-500/30 text-xs font-mono font-bold">
                      {bench.isBeeWinner
                        ? home.benchmarksFastest
                        : home.benchmarksParity}
                    </span>
                  </div>

                  {/* Multiplier Callout */}
                  <div className="mb-6 inline-flex items-center gap-1.5 px-3 py-1 rounded-lg bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/30 text-xs font-mono font-semibold">
                    <Zap className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
                    <span>{bench.multiplier}</span>
                  </div>

                  {/* Visual Horizontal Comparison Bars */}
                  <div className="space-y-4">
                    {/* Beejs Bar */}
                    <div>
                      <div className="flex items-center justify-between text-xs font-mono mb-1.5">
                        <span className="text-amber-800 dark:text-amber-300 font-bold flex items-center gap-1.5">
                          <span>Beejs {BEEJS_VERSION} (Rust)</span>
                          {bench.isBeeWinner ? (
                            <span className="text-[10px] px-1.5 py-0.2 rounded bg-amber-500/20 text-amber-800 dark:text-amber-300 font-bold">
                              #1
                            </span>
                          ) : null}
                        </span>
                        <span className="text-zinc-950 dark:text-white font-bold">
                          {bench.beeValue}{" "}
                          <span className="text-zinc-600 dark:text-zinc-400 font-normal">
                            ({bench.beeOps})
                          </span>
                        </span>
                      </div>
                      <div className="h-3.5 w-full bg-zinc-200 dark:bg-zinc-900 rounded-full overflow-hidden p-0.5 border border-amber-500/20">
                        <motion.div
                          initial={{ width: 0 }}
                          whileInView={{ width: `${bench.beeBar}%` }}
                          viewport={{ once: true }}
                          transition={{ duration: 0.8, ease: "easeOut" }}
                          className="h-full bg-gradient-to-r from-amber-500 to-amber-300 rounded-full shadow-sm shadow-amber-500/50"
                        />
                      </div>
                    </div>

                    {/* Bun Bar */}
                    <div>
                      <div className="flex items-center justify-between text-xs font-mono mb-1.5 text-zinc-700 dark:text-zinc-400">
                        <span>Bun 1.4.1 (Zig)</span>
                        <span className="text-zinc-900 dark:text-zinc-300">
                          {bench.bunValue}{" "}
                          <span className="text-zinc-600 dark:text-zinc-400">
                            ({bench.bunOps})
                          </span>
                        </span>
                      </div>
                      <div className="h-3 w-full bg-zinc-200 dark:bg-zinc-900 rounded-full overflow-hidden p-0.5 border border-zinc-300/80 dark:border-zinc-800">
                        <motion.div
                          initial={{ width: 0 }}
                          whileInView={{ width: `${bench.bunBar}%` }}
                          viewport={{ once: true }}
                          transition={{
                            duration: 0.8,
                            ease: "easeOut",
                            delay: 0.1,
                          }}
                          className="h-full bg-zinc-600 rounded-full"
                        />
                      </div>
                    </div>

                    {/* Node Bar */}
                    <div>
                      <div className="flex items-center justify-between text-xs font-mono mb-1.5 text-zinc-700 dark:text-zinc-400">
                        <span>Node.js v22 (C++)</span>
                        <span className="text-zinc-900 dark:text-zinc-300">
                          {bench.nodeValue}{" "}
                          <span className="text-zinc-600 dark:text-zinc-400">
                            ({bench.nodeOps})
                          </span>
                        </span>
                      </div>
                      <div className="h-3 w-full bg-zinc-200 dark:bg-zinc-900 rounded-full overflow-hidden p-0.5 border border-zinc-300/80 dark:border-zinc-800">
                        <motion.div
                          initial={{ width: 0 }}
                          whileInView={{ width: `${bench.nodeBar}%` }}
                          viewport={{ once: true }}
                          transition={{
                            duration: 0.8,
                            ease: "easeOut",
                            delay: 0.2,
                          }}
                          className="h-full bg-zinc-700/60 rounded-full"
                        />
                      </div>
                    </div>
                  </div>
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      </section>

      {/* Terminal Code Sandbox Preview with Tabs */}
      <section className="max-w-4xl mx-auto px-6 pb-28 relative z-10">
        <div className="text-center max-w-xl mx-auto mb-8">
          <h2 className="text-2xl sm:text-3xl font-extrabold text-zinc-950 dark:text-white font-display">
            Built for Modern Workloads
          </h2>
          <p className="mt-2 text-sm text-zinc-600 dark:text-zinc-400">
            Zero-config TypeScript 6.0, worker concurrency, and zero-overhead
            memory.
          </p>
        </div>

        <motion.div
          initial={{ opacity: 0, scale: 0.97 }}
          whileInView={{ opacity: 1, scale: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="glass-panel rounded-2xl overflow-hidden shadow-2xl border-zinc-800"
        >
          {/* Top Window Bar & Workload Tabs */}
          <div className="px-4 py-3 bg-zinc-900/90 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 rounded-full bg-red-500/80" />
              <div className="w-3 h-3 rounded-full bg-amber-500/80" />
              <div className="w-3 h-3 rounded-full bg-emerald-500/80" />
              <span className="ml-2 text-xs font-mono text-zinc-400 hidden sm:inline">
                {home.sandboxTitle}
              </span>
            </div>

            {/* Code Selector Tabs */}
            <div className="flex items-center gap-1.5 p-1 rounded-xl bg-zinc-950/60 border border-zinc-800">
              <button
                onClick={() => setActiveTab("http")}
                className={`px-3 py-1 rounded-lg text-xs font-mono font-medium transition-all cursor-pointer ${
                  activeTab === "http"
                    ? "bg-amber-500 text-zinc-950 font-bold"
                    : "text-zinc-400 hover:text-white"
                }`}
              >
                Multi-Worker HTTP
              </button>
              <button
                onClick={() => setActiveTab("ts")}
                className={`px-3 py-1 rounded-lg text-xs font-mono font-medium transition-all cursor-pointer ${
                  activeTab === "ts"
                    ? "bg-amber-500 text-zinc-950 font-bold"
                    : "text-zinc-400 hover:text-white"
                }`}
              >
                TypeScript 6.0
              </button>
              <button
                onClick={() => setActiveTab("buffer")}
                className={`px-3 py-1 rounded-lg text-xs font-mono font-medium transition-all cursor-pointer ${
                  activeTab === "buffer"
                    ? "bg-amber-500 text-zinc-950 font-bold"
                    : "text-zinc-400 hover:text-white"
                }`}
              >
                SIMD Buffer
              </button>
            </div>
          </div>

          {/* Code Viewer Panel */}
          <div className="p-6 font-mono text-sm leading-relaxed overflow-x-auto text-zinc-100 bg-[#0a0b0e] select-text">
            {activeTab === "http" && (
              <div>
                <div className="text-zinc-400 italic">
                  {home.sandboxComment}
                </div>
                <div>
                  <span className="text-purple-400">import</span> &#123;{" "}
                  <span className="text-blue-300 font-medium">
                    createServer
                  </span>{" "}
                  &#125; <span className="text-purple-400">from</span>{" "}
                  <span className="text-emerald-400">'node:http'</span>;
                </div>
                <br />
                <div>
                  <span className="text-zinc-400 italic">
                    // Connections dispatched across lock-free worker threads
                  </span>
                </div>
                <div>
                  <span className="text-blue-400">const</span>{" "}
                  <span className="text-amber-300">server</span> ={" "}
                  <span className="text-blue-400">createServer</span>((
                  <span className="text-amber-200">req</span>,{" "}
                  <span className="text-amber-200">res</span>) =&gt; &#123;
                </div>
                <div className="pl-4">
                  <span className="text-amber-200">res</span>.
                  <span className="text-blue-400">writeHead</span>(
                  <span className="text-orange-400">200</span>, &#123;{" "}
                  <span className="text-emerald-400">'Content-Type'</span>:{" "}
                  <span className="text-emerald-400">'application/json'</span>{" "}
                  &#125;);
                </div>
                <div className="pl-4">
                  <span className="text-amber-200">res</span>.
                  <span className="text-blue-400">end</span>(
                  <span className="text-purple-300">JSON</span>.
                  <span className="text-blue-400">stringify</span>(&#123;{" "}
                  <span className="text-amber-300">runtime</span>:{" "}
                  <span className="text-emerald-400">'beejs'</span>,{" "}
                  <span className="text-amber-300">version</span>:{" "}
                  <span className="text-emerald-400">'1.4.0'</span>,{" "}
                  <span className="text-amber-300">workers</span>:{" "}
                  <span className="text-orange-400">8</span> &#125;));
                </div>
                <div>&#125;);</div>
                <br />
                <div>
                  <span className="text-amber-300">server</span>.
                  <span className="text-blue-400">listen</span>(
                  <span className="text-orange-400">3000</span>, () =&gt; &#123;
                </div>
                <div className="pl-4 text-emerald-400">
                  <span className="text-purple-300">console</span>.
                  <span className="text-blue-400">log</span>(
                  <span className="text-emerald-400">'{home.sandboxLog}'</span>
                  );
                </div>
                <div>&#125;);</div>
              </div>
            )}

            {activeTab === "ts" && (
              <div>
                <div className="text-zinc-400 italic">
                  // Native oxc transpilation: Stage 3 Decorators, TSX &
                  Explicit Resource Management
                </div>
                <div>
                  <span className="text-purple-400">import</span>{" "}
                  <span className="text-amber-200">React</span>{" "}
                  <span className="text-purple-400">from</span>{" "}
                  <span className="text-emerald-400">'react'</span>;
                </div>
                <br />
                <div>
                  <span className="text-blue-400">interface</span>{" "}
                  <span className="text-cyan-400">BenchmarkResult</span> &#123;
                </div>
                <div className="pl-4">
                  <span className="text-amber-200">suite</span>:{" "}
                  <span className="text-purple-400">string</span>;
                </div>
                <div className="pl-4">
                  <span className="text-amber-200">throughput</span>:{" "}
                  <span className="text-purple-400">number</span>;
                </div>
                <div>&#125;</div>
                <br />
                <div>
                  <span className="text-blue-400">const</span>{" "}
                  <span className="text-amber-300">result</span>:{" "}
                  <span className="text-cyan-400">BenchmarkResult</span> =
                  &#123;
                </div>
                <div className="pl-4">
                  <span className="text-amber-200">suite</span>:{" "}
                  <span className="text-emerald-400">'require(module)'</span>,
                </div>
                <div className="pl-4">
                  <span className="text-amber-200">throughput</span>:{" "}
                  <span className="text-orange-400">4_601_226</span>,
                </div>
                <div>&#125;;</div>
                <br />
                <div className="text-emerald-400">
                  <span className="text-purple-300">console</span>.
                  <span className="text-blue-400">log</span>(`🚀
                  $&#123;result.suite&#125; -&gt; $&#123;result.throughput&#125;
                  ops/s`);
                </div>
              </div>
            )}

            {activeTab === "buffer" && (
              <div>
                <div className="text-zinc-400 italic">
                  // Zero-copy Rust SIMD buffer operations
                </div>
                <div>
                  <span className="text-purple-400">import</span> &#123;{" "}
                  <span className="text-purple-300">Buffer</span> &#125;{" "}
                  <span className="text-purple-400">from</span>{" "}
                  <span className="text-emerald-400">'node:buffer'</span>;
                </div>
                <br />
                <div>
                  <span className="text-blue-400">const</span>{" "}
                  <span className="text-amber-300">size</span> ={" "}
                  <span className="text-orange-400">64</span> *{" "}
                  <span className="text-orange-400">1024</span>;
                </div>
                <div>
                  <span className="text-blue-400">const</span>{" "}
                  <span className="text-amber-300">buf</span> ={" "}
                  <span className="text-purple-300">Buffer</span>.
                  <span className="text-blue-400">allocUnsafe</span>(
                  <span className="text-amber-300">size</span>);
                </div>
                <br />
                <div>
                  <span className="text-amber-300">buf</span>.
                  <span className="text-blue-400">fill</span>(
                  <span className="text-orange-400">0xaa</span>);
                </div>
                <div>
                  <span className="text-blue-400">const</span>{" "}
                  <span className="text-amber-300">slice</span> ={" "}
                  <span className="text-amber-300">buf</span>.
                  <span className="text-blue-400">subarray</span>(
                  <span className="text-orange-400">0</span>,{" "}
                  <span className="text-orange-400">1024</span>);
                </div>
                <br />
                <div className="text-emerald-400">
                  <span className="text-purple-300">console</span>.
                  <span className="text-blue-400">log</span>(`⚡ 100,000 SIMD
                  buffer ops in 2.09ms (47,846 ops/s)`);
                </div>
              </div>
            )}

            {/* Execution Result Bar */}
            <div className="mt-6 pt-4 border-t border-zinc-800/80 text-xs text-emerald-400 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                <span>
                  $ bee run{" "}
                  {activeTab === "http"
                    ? "server.ts"
                    : activeTab === "ts"
                      ? "app.tsx"
                      : "simd_buffer.ts"}
                </span>
              </div>
              <span className="text-zinc-300 font-mono text-[11px]">
                {home.sandboxBoot}
              </span>
            </div>
          </div>
        </motion.div>
      </section>

      {/* Core Capabilities / Architectural Highlights */}
      <section className="max-w-6xl mx-auto px-6 pb-28 relative z-10">
        <div className="text-center max-w-2xl mx-auto mb-14">
          <h2 className="text-3xl sm:text-4xl font-extrabold text-zinc-950 dark:text-white font-display">
            {home.featuresTitle}
          </h2>
          <p className="mt-3 text-zinc-700 dark:text-zinc-400 text-base">
            {home.featuresSubtitle}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {home.features.map((feature, idx) => (
            <FeatureCard
              key={feature.title}
              icon={featureIcons[idx]}
              title={feature.title}
              desc={feature.desc}
            />
          ))}
        </div>
      </section>

      {/* Runtime Subsystems Grid */}
      <section className="max-w-6xl mx-auto px-6 pb-28 relative z-10">
        <div className="text-center max-w-2xl mx-auto mb-14">
          <span className="text-xs font-mono uppercase tracking-widest text-amber-700 dark:text-amber-400 font-semibold">
            {home.systemsMeta}
          </span>
          <h2 className="text-3xl sm:text-4xl font-extrabold text-zinc-950 dark:text-white font-display mt-2">
            {home.systemsTitle}
          </h2>
          <p className="mt-3 text-zinc-700 dark:text-zinc-400 text-base">
            {home.systemsSubtitle}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
          {home.systems.map((sub, idx) => (
            <div
              key={sub.title}
              className="glass-card rounded-2xl p-6 border-zinc-200/80 dark:border-zinc-800/80 hover:border-amber-500/30 transition-all group"
            >
              <div className="w-10 h-10 rounded-xl bg-zinc-100 dark:bg-zinc-800/80 border border-zinc-200 dark:border-zinc-700/60 flex items-center justify-center mb-4 group-hover:bg-amber-500/10 group-hover:border-amber-500/30 transition-all">
                {subsystemIcons[idx]}
              </div>
              <h3 className="text-base font-bold text-zinc-950 dark:text-white font-display group-hover:text-amber-600 dark:group-hover:text-amber-300 transition-colors">
                {sub.title}
              </h3>
              <p className="mt-2 text-xs sm:text-sm text-zinc-700 dark:text-zinc-400 leading-relaxed font-normal">
                {sub.desc}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Bottom CTA Banner */}
      <section className="max-w-5xl mx-auto px-6 relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="relative rounded-3xl glass-panel border-amber-500/30 p-10 sm:p-14 text-center overflow-hidden glow-amber-lg"
        >
          <div className="ambient-glow top-0 left-1/2 -translate-x-1/2 w-96 h-60 bg-amber-500/20 pointer-events-none" />

          <h2 className="text-3xl sm:text-4xl md:text-5xl font-extrabold text-zinc-950 dark:text-white font-display tracking-tight relative z-10">
            {home.ctaTitle}
          </h2>

          <p className="mt-4 text-base sm:text-lg text-zinc-700 dark:text-zinc-300 max-w-xl mx-auto relative z-10 font-normal">
            {home.ctaSubtitle}
          </p>

          <div className="mt-8 flex flex-col sm:flex-row items-center justify-center gap-4 relative z-10">
            <Link
              to="/docs"
              className="w-full sm:w-auto px-8 py-3.5 rounded-full bg-amber-500 text-zinc-950 font-bold text-sm hover:bg-amber-400 transition-all flex items-center justify-center gap-2 shadow-lg shadow-amber-500/30"
            >
              <span>{home.ctaButton}</span>
              <ArrowRight className="w-4 h-4" />
            </Link>
            <Link
              to={home.latestArticle.link}
              className="w-full sm:w-auto px-8 py-3.5 rounded-full glass-card hover:bg-zinc-200/80 dark:hover:bg-zinc-800/60 text-zinc-800 dark:text-zinc-200 font-semibold text-sm transition-all border border-zinc-300/80 dark:border-zinc-700/60"
            >
              <span>{home.ctaNotesButton}</span>
            </Link>
          </div>
        </motion.div>
      </section>
    </div>
  );
}

function FeatureCard({
  icon,
  title,
  desc,
}: {
  icon: React.ReactNode;
  title: string;
  desc: string;
}) {
  return (
    <div className="glass-card rounded-2xl p-7 border-zinc-200/80 dark:border-zinc-800/80 group hover:border-amber-500/30 transition-all">
      <div className="w-12 h-12 rounded-xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center mb-5 group-hover:scale-110 group-hover:bg-amber-500/20 transition-all">
        {icon}
      </div>
      <h3 className="text-lg font-bold text-zinc-950 dark:text-white font-display group-hover:text-amber-600 dark:group-hover:text-amber-300 transition-colors">
        {title}
      </h3>
      <p className="mt-2.5 text-sm text-zinc-700 dark:text-zinc-400 leading-relaxed font-normal">
        {desc}
      </p>
    </div>
  );
}
