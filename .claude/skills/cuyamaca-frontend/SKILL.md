---
name: cuyamaca-frontend
description: >
  Design and build frontend interfaces for the Cuyamaca project using its exact design language:
  warm white liquid glass surfaces, sunset color palette drawn from Mt. Cuyamaca (amber gold, dusty
  rose, violet-slate, deep sky blue, granite white), and an industrial-technical aesthetic softened
  by warm natural light. Use this skill whenever building or styling any Cuyamaca UI component,
  view, or artifact — including the project editor, runtime window, parts panel, code view, serial
  monitor, settings, or any new surface. Also trigger when the user asks about Cuyamaca's color
  scheme, design tokens, glass treatment, or visual language. ALWAYS consult this skill before
  writing any Cuyamaca CSS, Tailwind classes, or component markup.
---

# Cuyamaca Frontend Design Skill

Cuyamaca is a Tauri v2 desktop app for natural language Arduino/robotics control. Its UI uses a
**warm-white liquid glass** design language — a light base (not dark) with glass surfaces that
refract a sunset color palette. This is the visual counterpart to Sierra (which uses dark glass);
Cuyamaca uses the opposite pole: bleached granite catching late afternoon sun.

Read `references/palette.md` for exact color values and token names.
Read `references/glass.md` for CSS/Tailwind implementations of each glass tier.
Read `references/components.md` for specific component patterns (device cards, serial monitor, etc).

---

## Design Philosophy

The photo that inspired this design: Mt. Cuyamaca at sunset in fall. Warm golden grasslands,
granite boulders, rose-violet clouds, deep slate-blue sky, dark silhouetted ridgelines. The UI
should feel like that boulder in the foreground — warm white stone, weathered and solid, with
the rich sunset colors glowing *behind* and *through* it, not on top of it.

**Key visual ideas:**
- Glass surfaces are warm white / cream, not dark. The "liquid glass" effect comes from a warm
  luminous backing, not a dark void.
- Color lives in the gradients behind glass, not in the glass itself. The panels are translucent
  cream; the richness comes from what bleeds through.
- Technical precision (monospace code, sensor values, serial output) coexists with organic warmth.
  The data reads like something etched into stone, not rendered on a screen.
- Depth is created by layering warm amber glows and violet-blue shadows, not by dark backgrounds.

---

## Color Palette

All values defined as CSS custom properties. Full reference in `references/palette.md`.

### Background Canvas
The app background is a warm gradient — not pure white, not dark. Think bleached grassland under
a pink-gold sky.

```css
--bg-base:        #FBF6EE;   /* warm parchment — the ground */
--bg-gradient:    radial-gradient(ellipse at 70% 20%, #F2C9A0 0%, #EDD5C0 25%, #E8DDD5 50%, #D8D4E8 80%, #C4C8DC 100%);
/* amber-gold upper right fading to violet-blue lower left — the sky */
```

### Palette Tokens

| Token | Hex | Source in photo |
|---|---|---|
| `--amber-gold` | `#D4843A` | golden grassland |
| `--amber-light` | `#F0B870` | lit grass highlights |
| `--amber-glow` | `rgba(240,184,112,0.35)` | warm ambient fill |
| `--rose-dust` | `#C4748A` | rose clouds |
| `--rose-light` | `#E8A8B8` | lighter cloud edges |
| `--violet-slate` | `#7A7AA8` | violet-blue sky band |
| `--violet-deep` | `#4A4A78` | deep upper sky |
| `--sky-blue` | `#8090B4` | evening sky blue |
| `--granite-white` | `#F4F0EC` | boulder surface |
| `--granite-warm` | `#E8E0D8` | boulder shadow side |
| `--ridgeline` | `#3A3848` | dark silhouetted hills |
| `--earth-brown` | `#8A6840` | soil / dark rock |

### Text Colors (on warm-white glass)
```css
--text-primary:    #2C2830;   /* near-black with violet undertone — ridgeline */
--text-secondary:  #7A6858;   /* warm brown-grey */
--text-tertiary:   #A89888;   /* muted warm grey */
--text-accent:     #8A6840;   /* earth brown — labels, section headers */
--text-code:       #4A4478;   /* violet-slate — monospace data */
--text-sensor:     #6A5020;   /* deep amber — live sensor readouts */
```

---

## Glass Hierarchy

Three tiers of translucency on the warm-white light base. Details in `references/glass.md`.

### Glass Subtle — structural chrome
Sidebar, panel headers, status bar. Lightest, most transparent.
```css
backdrop-filter: blur(16px) saturate(1.4);
background: rgba(252, 246, 238, 0.55);
border: 0.5px solid rgba(212, 180, 140, 0.30);
box-shadow: 0 1px 0 rgba(255,255,255,0.6) inset, 0 4px 16px rgba(120,100,60,0.06);
```

### Glass Standard — interactive elements
Component cards, input fields, chat bubbles, code blocks.
```css
backdrop-filter: blur(24px) saturate(1.6);
background: rgba(252, 246, 238, 0.72);
border: 0.5px solid rgba(212, 180, 140, 0.45);
box-shadow: 0 1px 0 rgba(255,255,255,0.8) inset, 0 4px 20px rgba(120,100,60,0.08);
```

### Glass Strong — emphasis
Modals, active selections, focused states, user messages.
```css
backdrop-filter: blur(32px) saturate(1.8);
background: rgba(252, 246, 238, 0.88);
border: 0.5px solid rgba(212, 180, 140, 0.60);
box-shadow: 0 1px 0 rgba(255,255,255,0.9) inset, 0 8px 32px rgba(120,100,60,0.12);
```

### Glass Tinted variants
Used for accents — always apply on top of the standard glass tier:
```css
/* Amber tinted — motor activity, warnings */
background: rgba(240, 184, 112, 0.18);
border-color: rgba(212, 132, 58, 0.35);

/* Rose tinted — user messages, primary actions */  
background: rgba(196, 116, 138, 0.14);
border-color: rgba(196, 116, 138, 0.30);

/* Violet tinted — code model responses, AI messages */
background: rgba(122, 122, 168, 0.12);
border-color: rgba(122, 122, 168, 0.25);

/* Green tinted — success, healthy connections */
background: rgba(93, 160, 120, 0.14);
border-color: rgba(93, 160, 120, 0.30);

/* Red tinted — errors, kill button, disconnected */
background: rgba(180, 80, 70, 0.12);
border-color: rgba(180, 80, 70, 0.28);
```

---

## Typography

```css
/* Proportional — prose, labels, chat */
font-family: 'Freight Text Pro', 'Palatino Linotype', Georgia, 'Times New Roman', serif;
/* Fallback: Georgia is a warm, slightly editorial serif that fits the natural aesthetic */

/* Monospace — code, serial, sensor values */
font-family: 'Triplicate T4c', 'Courier Prime', 'Courier New', monospace;
/* Fallback: Courier Prime has warmth; avoid cold monospace like JetBrains here */
```

| Role | Size | Weight | Color |
|---|---|---|---|
| Body prose | 13.5px | 400 | `--text-primary` |
| Labels | 11px | 500 | `--text-accent`, uppercase, tracking 0.07em |
| Section headers | 14px | 500 | `--text-primary` |
| Serial output | 11px mono | 400 | `--text-secondary` |
| Sensor values | 13px mono | 500 | `--text-sensor` |
| Code | 13px mono | 400 | `--text-code` |
| Large readouts | 22px | 300 | `--text-primary` |

---

## Accent Usage Rules

Unlike Sierra's dark glass (where accents pop against darkness), on warm-white glass accents must
be used more sparingly to avoid muddiness.

- **Amber-gold**: Active states, motor/actuator activity, section highlights. Use at ≤40% opacity for backgrounds.
- **Rose-dust**: User messages, primary CTA, flashing/compiling state. Use at ≤30% opacity for backgrounds.
- **Violet-slate**: AI/model responses, code annotation, secondary actions. Use at ≤25% opacity for backgrounds.
- **Green** (`#5DA078`): Success, healthy, approved. Never use amber + green together — they clash on warm base.
- **Red** (`#B45046`): Errors, kill, rejected, disconnected. Use full opacity only for the kill button track.

Status dots: 6px, `box-shadow: 0 0 5px currentColor` at 60% opacity. On light backgrounds, dots
need a subtle dark ring: `outline: 1px solid rgba(0,0,0,0.08)`.

---

## Ambient Background

The background gradient behind all glass panels is load-bearing. It's what the glass refracts.

```css
.app-background {
  background: var(--bg-base);
  background-image:
    radial-gradient(ellipse 80% 60% at 75% 15%, rgba(240,184,112,0.50) 0%, transparent 60%),
    radial-gradient(ellipse 50% 70% at 15% 80%, rgba(122,122,168,0.30) 0%, transparent 55%),
    radial-gradient(ellipse 60% 40% at 50% 50%, rgba(196,116,138,0.15) 0%, transparent 50%);
  /* Result: warm amber glow top-right (sun), violet-blue bloom bottom-left (sky reflection),
     faint rose center (cloud ambient) */
}
```

The gradient orbs use `animation: drift 18s ease-in-out infinite alternate` — extremely slow,
subliminal. Wrap in `@media (prefers-reduced-motion: no-preference)`.

---

## Layout

Three-panel structure matching Sierra. See `references/components.md` for specifics.

```
┌──────────┬────────────────────────────┬─────────────────┐
│ Sidebar  │ Main Area (flex: 1)        │ Parts Panel     │
│ (220px)  │                            │ (260px)         │
│          │ [Tab: Manifest|Code|Chat]  │                 │
│ Projects │                            │ Component list  │
│          │ View content               │ Pin editor      │
│ ─────── │                            │                 │
│ Health   │ Input capsule (Chat only)  │                 │
└──────────┴────────────────────────────┴─────────────────┘
```

All three panels use Glass Subtle. The main area content (code, cards, messages) uses Glass
Standard and Glass Strong. The background gradient shows through all panels.

---

## Key Differentiators from Sierra

| Property | Sierra (dark glass) | Cuyamaca (warm-white glass) |
|---|---|---|
| Base | Deep dark `#0D0D14` | Warm parchment `#FBF6EE` |
| Glass bg | `rgba(255,255,255,0.05–0.12)` | `rgba(252,246,238,0.55–0.88)` |
| Border | White at low opacity | Warm amber at low opacity |
| Accent primacy | Purple, Teal | Rose-dust, Amber-gold |
| Text | White at varying opacity | Near-black with warm/violet undertones |
| Gradient backing | Cool purples, teals | Warm ambers, dusty rose, violet-slate |
| Monospace color | Cyan glow | Warm amber / violet-slate |
| Feel | Night sky, tech | Stone, earth, warm engineering |

---

## Animation Principles

Same restraint as Sierra. All timing identical unless noted.

- **Message entry**: 280ms ease-out, translateY(6px) → 0, opacity 0 → 1. Slightly slower than
  Sierra — the warm aesthetic benefits from a slightly more deliberate pace.
- **Toggle**: 250ms. On track: amber-gold tint. Off track: granite-warm.
- **Action pill pulse**: Single amber glow pulse for success, red for failure.
- **Background drift**: 18s, slower than Sierra's 12-15s. More geological, less digital.
- **Code diff highlight**: Brief amber flash on added lines, rose flash on removed lines.
- **Sensor value flash**: Amber pulse on value change in sensor state panel.

---

## Accessibility

Same rules as Sierra. Warm-white glass is naturally higher contrast than dark glass, but still
requires attention:

- Minimum 4.5:1 contrast for body text. `#2C2830` on `rgba(252,246,238,0.88)` ≈ 11:1. ✓
- Test amber text (`--text-sensor` `#6A5020`) against the lightest possible glass background.
- "Reduce transparency" mode: replace glass with `#F5EFE8` solid. Keep layout identical.
- Focus rings: 2px solid `--rose-dust`, offset 2px.
- Respect `prefers-color-scheme: dark` — if the OS is in dark mode, consider switching to
  Sierra's dark glass language automatically, since warm-white glass on a dark desktop looks wrong.

---

## What NOT to Do

- Do not use pure white (`#FFFFFF`) surfaces. The base is warm parchment, not sterile white.
- Do not use cool greys. Every neutral has a warm undertone.
- Do not mix amber and green as equal-weight accents — amber dominates on this base.
- Do not make the serial monitor or code blocks look clinical. Warm the monospace. The data is
  from organic hardware in the real world.
- Do not apply backdrop-filter to the animated background orbs themselves — only to foreground
  glass panels.
- Do not use purple as a dominant color. It exists in the sky/gradient backing, not in the UI
  chrome. It should bleed through glass subtly, not be applied directly.
- Do not fight the warmth. Every instinct to reach for a cool, sterile component should be
  redirected toward the palette.