# Beejs website visual world

Canon: Deno + Vite + Bun, played straight. Light default. Tech feel lives in the terminal, not in glow.

## Color

| Token | Light | Dark |
| --- | --- | --- |
| Paper | `#F5F5F4` | `#0C0C0B` |
| Ink | `#18181B` | `#FAFAF9` |
| Muted | `#52525B` | `#A1A1AA` |
| Line | `#E4E4E7` | `#27272A` |
| Honey | `#F0C000` | `#F0C000` |
| Honey ink | `#14120B` | `#14120B` |
| Code bg | `#18181B` | `#111110` |

Honey (`#F0C000`) is a **field**, not a pinstripe: the home hero sits on it. Ink on honey is `#14120B`. The black terminal is the other large surface. Links are ink, underline on hover. No gradient text.

## Type

- UI: Source Sans 3 + Noto Sans SC
- Code: JetBrains Mono
- Home display: clamp 2.5–4.25rem, weight 700. Tracking ≥ -0.03em.
- Body measure ~68ch.

## Layout

- Max width 960px for reading; 1080px for tables.
- Left-aligned hero. Install command is the first control.
- Sticky header: paper background, 1px bottom line, no glass, no pill nav.
- Buttons: 6px radius, black fill or 1px border. Not pills.

## Motion

- None on load. Hover: color and underline. `prefers-reduced-motion` respected.

## Refuse

Glass, blur orbs, cyber grids, sparkles, 8xl headlines, identical icon cards, metric-hero templates.
