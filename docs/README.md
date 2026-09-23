# Amber Documentation

Start here when deciding what Amber can do today and where it is headed:

- [Current Scope](CURRENT_SCOPE.md): Stable, Preview, Experimental, and Historical capability boundaries.
- [v1.9.1 implementation plan](IMPLEMENTATION_PLAN_v1.9.1.md): P0–P2 infrastructure second wave (Windows artifacts, Homebrew SHA, HTTPS serve, inspector, N-API hello, CI honesty). Plan only; not current product fact.
- [Infrastructure backlog](INFRASTRUCTURE_CAPABILITY_BACKLOG.md): first-wave runtime supporting facilities and what remains.
- [Three-Year Technical Roadmap (2026-2029)](THREE_YEAR_ROADMAP_2026_2029.md): Bold technical vision for AI-Native, Edge-Native, Polyglot, and Sovereign Sandbox evolution.
- [Quick Start](QUICK_START.md): source-first build and first smoke commands.
- [CLI Usage Guide](CLI_USAGE_GUIDE.md): command syntax and examples. Cross-check command maturity with Current Scope.

## Historical Material

This repository contains many stage plans, completion reports, benchmark notes, and archived progress logs. They are useful for understanding how the project evolved, but they are not current product capability proof.

Treat these as historical unless a current-status document explicitly revalidates them:

- `STAGE_*`
- `IMPLEMENTATION_PLAN_STAGE_*`
- stage completion reports
- stage benchmark reports
- archived progress logs

When historical material conflicts with `Cargo.toml`, `src/lib.rs`, `src/main.rs`, executable tests, or [Current Scope](CURRENT_SCOPE.md), the current fact sources win.
