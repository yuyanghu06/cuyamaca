---
name: cuyamaca-scaffold
description: Scaffold a Tauri v2 desktop app for the Cuyamaca project — an Arduino robotics controller with natural language control. Use this skill whenever the user wants to initialize the Cuyamaca project, set up the Tauri v2 scaffold, create the base layout and warm-white liquid glass UI theme, verify the IPC bridge, or references "phase 1", "scaffold", "project setup", "initialize Cuyamaca", "create the app skeleton", or "base layout". Also trigger when the user asks about Cuyamaca's three-panel layout, the warm-white glass design language, or setting up the Tauri project structure from scratch.
---

# Phase 1 — Tauri v2 Scaffold + IPC Verification

This skill creates the foundational Tauri v2 project for Cuyamaca: a desktop app for natural language control of Arduino-based robotics. By the end, you have a building Tauri v2 app with the warm-white liquid glass UI skeleton, three-panel layout, navigation, and a verified IPC bridge between the React frontend and Rust backend.

## What This Skill Produces

- A Tauri v2 project with React + TypeScript frontend and Rust backend
- The three-panel layout: sidebar (220px), main area (flex), parts panel (260px)
- Warm-white liquid glass theme with CSS variables, glass hierarchy, sunset palette, and ambient gradient background
- Sidebar with navigation (Manifest, Code, Chat) and service health indicators
- A `ping` Tauri command that round-trips a string from frontend to backend and back — proving IPC works
- Project directory structure for both Rust (`src-tauri/src/commands/`, `src-tauri/src/services/`) and TypeScript (`src/commands/`, `src/components/`, `src/views/`)

## Prerequisites

- Node.js (v18+) and npm
- Rust toolchain (rustup, cargo)
- Tauri v2 CLI (`cargo install tauri-cli --version "^2"`)
- A code editor

## Step 1: Initialize the Tauri v2 Project

Use the Tauri v2 scaffolder:

```bash
npm create tauri-app@latest cuyamaca -- --template react-ts
cd cuyamaca
npm install
```

This gives you `src/` (React frontend) and `src-tauri/` (Rust backend). Verify it builds:

```bash
npm run tauri dev
```

You should see an empty Tauri window. Kill it and proceed.

## Step 2: Establish the Directory Structure

Organize both sides of the app for the full build:

```
src/
├── commands/          # Tauri command wrappers (TS calls to Rust)
│   └── index.ts
├── components/        # Reusable UI components
│   ├── Sidebar.tsx
│   ├── PartsPanel.tsx
│   ├── StatusDot.tsx
│   └── GlassPanel.tsx
├── views/             # Full-page views swapped in main area
│   ├── ManifestView.tsx
│   ├── CodeView.tsx
│   └── ChatView.tsx
├── styles/
│   └── globals.css    # Theme variables, glass classes, base styles
├── App.tsx
└── main.tsx

src-tauri/src/
├── commands/          # Tauri command handlers
│   ├── mod.rs
│   └── debug.rs       # ping command for IPC verification
├── services/          # External service abstractions
│   └── mod.rs
├── lib.rs             # Tauri app builder, state registration, command registration
└── main.rs            # Entry point
```

Create all directories and stub files now. Views can render placeholder text ("Manifest View", "Code View", "Chat View") — they get real content in later phases.

## Step 3: Define the Warm-White Liquid Glass Theme

Cuyamaca's design language is warm, natural, and precision-focused — inspired by Mt. Cuyamaca at sunset in fall. Warm golden grasslands, granite boulders, rose-violet clouds, deep slate-blue sky. Glass surfaces are warm white / cream, not dark. The richness comes from what bleeds through the translucent panels — a sunset gradient backing — not from the surfaces themselves.

This is NOT the same as Sierra's dark glass aesthetic. Cuyamaca uses the opposite pole: bleached granite catching late afternoon sun.

Create `src/styles/globals.css` with these CSS variables:

```css
:root {
  /* background canvas */
  --bg-base: #FBF6EE;             /* warm parchment — the ground */
  --bg-gradient-amber: rgba(240, 184, 112, 0.50);
  --bg-gradient-violet: rgba(122, 122, 168, 0.30);
  --bg-gradient-rose: rgba(196, 116, 138, 0.15);

  /* glass hierarchy — warm white translucency */
  --glass-subtle-bg: rgba(252, 246, 238, 0.55);
  --glass-subtle-border: rgba(212, 180, 140, 0.30);
  --glass-subtle-blur: 16px;
  --glass-subtle-shadow: 0 1px 0 rgba(255,255,255,0.6) inset, 0 4px 16px rgba(120,100,60,0.06);

  --glass-standard-bg: rgba(252, 246, 238, 0.72);
  --glass-standard-border: rgba(212, 180, 140, 0.45);
  --glass-standard-blur: 24px;
  --glass-standard-shadow: 0 1px 0 rgba(255,255,255,0.8) inset, 0 4px 20px rgba(120,100,60,0.08);

  --glass-strong-bg: rgba(252, 246, 238, 0.88);
  --glass-strong-border: rgba(212, 180, 140, 0.60);
  --glass-strong-blur: 32px;
  --glass-strong-shadow: 0 1px 0 rgba(255,255,255,0.9) inset, 0 8px 32px rgba(120,100,60,0.12);

  /* palette — sunset over Mt. Cuyamaca */
  --amber-gold: #D4843A;          /* golden grassland */
  --amber-light: #F0B870;         /* lit grass highlights */
  --amber-glow: rgba(240, 184, 112, 0.35);  /* warm ambient fill */

  --rose-dust: #C4748A;           /* rose clouds */
  --rose-light: #E8A8B8;          /* lighter cloud edges */

  --violet-slate: #7A7AA8;        /* violet-blue sky band */
  --violet-deep: #4A4A78;         /* deep upper sky */

  --sky-blue: #8090B4;            /* evening sky blue */

  --granite-white: #F4F0EC;       /* boulder surface */
  --granite-warm: #E8E0D8;        /* boulder shadow side */

  --ridgeline: #3A3848;           /* dark silhouetted hills */
  --earth-brown: #8A6840;         /* soil / dark rock */

  /* accent tint backgrounds — layered on top of standard glass */
  --tint-amber-bg: rgba(240, 184, 112, 0.18);
  --tint-amber-border: rgba(212, 132, 58, 0.35);

  --tint-rose-bg: rgba(196, 116, 138, 0.14);
  --tint-rose-border: rgba(196, 116, 138, 0.30);

  --tint-violet-bg: rgba(122, 122, 168, 0.12);
  --tint-violet-border: rgba(122, 122, 168, 0.25);

  --tint-green-bg: rgba(93, 160, 120, 0.14);
  --tint-green-border: rgba(93, 160, 120, 0.30);
  --green: #5DA078;

  --tint-red-bg: rgba(180, 80, 70, 0.12);
  --tint-red-border: rgba(180, 80, 70, 0.28);
  --red: #B45046;

  /* text — warm on light glass */
  --text-primary: #2C2830;        /* near-black with violet undertone */
  --text-secondary: #7A6858;      /* warm brown-grey */
  --text-tertiary: #A89888;       /* muted warm grey */
  --text-accent: #8A6840;         /* earth brown — labels, section headers */
  --text-code: #4A4478;           /* violet-slate — monospace data */
  --text-sensor: #6A5020;         /* deep amber — live sensor readouts */

  /* typography */
  --font-mono: 'Triplicate T4c', 'Courier Prime', 'Courier New', monospace;
  --font-ui: 'Freight Text Pro', 'Palatino Linotype', Georgia, 'Times New Roman', serif;

  /* spacing */
  --sidebar-width: 220px;
  --parts-panel-width: 260px;
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
  --radius-capsule: 24px;

  /* reduced transparency fallback */
  --solid-surface: #F5EFE8;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: var(--bg-base);
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: 13.5px;
  line-height: 1.55;
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
}
```

### Ambient Background

The gradient behind all glass panels is load-bearing — it is what the glass refracts. Without it, the warm-white panels look flat.

```css
.app-background {
  position: fixed;
  inset: 0;
  background: var(--bg-base);
  background-image:
    radial-gradient(ellipse 80% 60% at 75% 15%, var(--bg-gradient-amber) 0%, transparent 60%),
    radial-gradient(ellipse 50% 70% at 15% 80%, var(--bg-gradient-violet) 0%, transparent 55%),
    radial-gradient(ellipse 60% 40% at 50% 50%, var(--bg-gradient-rose) 0%, transparent 50%);
  z-index: -1;
}

/* Extremely slow subliminal drift on the gradient orbs */
@media (prefers-reduced-motion: no-preference) {
  .app-background {
    animation: drift 18s ease-in-out infinite alternate;
  }

  @keyframes drift {
    0% { background-position: 0% 0%; }
    100% { background-position: 2% 3%; }
  }
}
```

### Glass Utility Classes

```css
.glass-subtle {
  backdrop-filter: blur(var(--glass-subtle-blur)) saturate(1.4);
  background: var(--glass-subtle-bg);
  border: 0.5px solid var(--glass-subtle-border);
  box-shadow: var(--glass-subtle-shadow);
}

.glass-standard {
  backdrop-filter: blur(var(--glass-standard-blur)) saturate(1.6);
  background: var(--glass-standard-bg);
  border: 0.5px solid var(--glass-standard-border);
  box-shadow: var(--glass-standard-shadow);
}

.glass-strong {
  backdrop-filter: blur(var(--glass-strong-blur)) saturate(1.8);
  background: var(--glass-strong-bg);
  border: 0.5px solid var(--glass-strong-border);
  box-shadow: var(--glass-strong-shadow);
}

/* Reduced transparency mode — solid warm surface, same layout */
@media (prefers-reduced-transparency: reduce) {
  .glass-subtle,
  .glass-standard,
  .glass-strong {
    backdrop-filter: none;
    background: var(--solid-surface);
    box-shadow: 0 1px 3px rgba(0,0,0,0.06);
  }
}
```

### Typography Classes

```css
.label {
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: var(--text-accent);
}

.mono {
  font-family: var(--font-mono);
}

.text-secondary {
  color: var(--text-secondary);
  font-size: 12px;
}

.text-sensor {
  font-family: var(--font-mono);
  font-weight: 500;
  color: var(--text-sensor);
  font-size: 13px;
}

.text-code {
  font-family: var(--font-mono);
  color: var(--text-code);
  font-size: 13px;
}
```

## Step 4: Build the Three-Panel Layout

The `App.tsx` renders the three-panel layout with CSS grid or flexbox:

```
┌──────────┬────────────────────────────┬─────────────────┐
│ Sidebar  │      Main Area             │  Parts Panel    │
│ (220px)  │      (flex: 1)             │  (260px)        │
└──────────┴────────────────────────────┴─────────────────┘
```

The sidebar is Glass Subtle. The parts panel is Glass Subtle and collapsible. The main area has no glass — it is the content canvas where the ambient background gradient shows through directly. Glass Standard and Glass Strong are used for content elements within the main area (cards, messages, code blocks).

App.tsx manages:
- Current active view (manifest / code / chat) — via React state
- Parts panel visibility — collapsible via state toggle
- View switching from sidebar navigation clicks

### Sidebar Component

The sidebar contains:

**Top section:** App name "Cuyamaca" in label style, earth-brown text.

**Navigation section:** Three nav items — Manifest, Code, Chat. Each is a clickable row that highlights when active. Active state uses a left border accent in `--amber-gold` and slightly brighter text (`--text-primary`). Inactive items use `--text-secondary`.

**Bottom section:** Service health indicators, stacked vertically:
- Ollama — status dot + label
- arduino-cli — status dot + label
- Code Model — status dot + label (shows model name when configured)
- Runtime Model — status dot + label (shows model name when configured)

All dots default to red (not connected) for now. They get wired to real health checks in later phases.

### StatusDot Component

A reusable 6px circle with a matching color glow and a subtle dark outline for visibility on the light background:

```css
.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  outline: 1px solid rgba(0, 0, 0, 0.08);
}

.status-dot.green {
  background: var(--green);
  box-shadow: 0 0 5px rgba(93, 160, 120, 0.6);
}

.status-dot.amber {
  background: var(--amber-gold);
  box-shadow: 0 0 5px rgba(212, 132, 58, 0.6);
}

.status-dot.red {
  background: var(--red);
  box-shadow: 0 0 5px rgba(180, 80, 70, 0.6);
}
```

### Parts Panel

Placeholder for now. Shows a "Components" header (label style, `--text-accent` earth-brown) and a message like "no project loaded" in `--text-secondary`. This gets real content in Phase 3 (project system + manifest editor). The panel is collapsible — a toggle button in the titlebar area hides/shows it.

### Responsive Behavior

- **≥1100px:** Full three-panel layout
- **900–1099px:** Parts panel collapses to an icon-toggle in the titlebar
- **<900px:** Sidebar collapses to icons only (no labels), parts panel hidden

## Step 5: Create the IPC Verification Command

This is the smoke test that proves the Tauri IPC bridge works.

### Rust side (`src-tauri/src/commands/debug.rs`)

```rust
#[tauri::command]
pub fn ping(message: String) -> Result<String, String> {
    Ok(format!("pong: {}", message))
}
```

Register it in `lib.rs`:

```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![commands::debug::ping])
    // ...
```

### TypeScript side (`src/commands/index.ts`)

```typescript
import { invoke } from "@tauri-apps/api/core";

export async function ping(message: string): Promise<string> {
  return invoke<string>("ping", { message });
}
```

### Verification in the UI

In one of the placeholder views (or a temporary dev panel), add a button that calls `ping("hello from frontend")` and displays the result. If you see "pong: hello from frontend" rendered in the UI, the IPC bridge is verified.

Remove the dev panel after verification or gate it behind a dev flag.

## Step 6: Configure Tauri Window Settings

In `src-tauri/tauri.conf.json`, set sensible defaults:

```json
{
  "app": {
    "windows": [
      {
        "title": "Cuyamaca",
        "width": 1200,
        "height": 800,
        "minWidth": 700,
        "minHeight": 500,
        "decorations": true,
        "transparent": false
      }
    ]
  }
}
```

The window is opaque — the warm parchment background is the base, not OS transparency. `decorations: true` uses the native title bar for now (draggable, close/minimize/maximize). A custom titlebar can come in later polish phases if desired.

## Step 7: Verify the Full Scaffold

Run `npm run tauri dev` and verify:

1. The app window opens at 1200×800 with a warm parchment background and visible sunset gradient orbs (amber upper-right, violet lower-left, faint rose center)
2. The three-panel layout renders: sidebar (left), main area (center), parts panel (right)
3. Sidebar shows nav items (Manifest, Code, Chat) and service health indicators (all red dots with subtle dark outlines)
4. Glass panels (sidebar, parts panel) appear as warm translucent cream surfaces — the gradient bleeds through them subtly
5. Clicking nav items switches the main area view (placeholder text changes)
6. Active nav item shows amber-gold left border accent
7. The parts panel shows "no project loaded" placeholder in warm brown-grey text
8. The parts panel can be collapsed/expanded
9. The IPC ping test returns "pong: ..." in the UI
10. Resizing the window below 1100px collapses the parts panel, below 900px collapses sidebar labels
11. Text is warm-toned (near-black with violet undertone on light surfaces), never cool grey or pure black

If all pass, Phase 1 is complete.

## What NOT to Do

- Do not use Sierra's dark color palette or cool-toned accents. Cuyamaca is warm-white liquid glass — parchment base, sunset palette, warm neutrals.
- Do not use pure white (`#FFFFFF`) surfaces. The base is warm parchment `#FBF6EE`, not sterile white.
- Do not use cool greys. Every neutral has a warm undertone.
- Do not add Ollama, arduino-cli, or serial port logic. Those come in later phases.
- Do not build the manifest editor, code editor, or chat interface. Those are later phases. Placeholder views only.
- Do not use custom title bars or frameless windows. Keep native decorations for now.
- Do not install unnecessary dependencies. Phase 1 needs only what Tauri v2 scaffolds plus React.
- Do not add a dark mode. The warm-white liquid glass theme is the only theme — though if the OS is in dark mode, a future polish phase may consider switching to Sierra's dark glass language automatically.
- Do not use cyan as a primary accent. Cyan belongs to Sierra's dark palette. Cuyamaca's primary accents are amber-gold and rose-dust.
- Do not mix amber and green as equal-weight accents — amber dominates on this warm base and they clash when used together.
- Do not apply backdrop-filter to the animated background orbs themselves — only to foreground glass panels.
- Do not use purple as a dominant UI chrome color. It exists in the sky/gradient backing and bleeds through glass subtly, but is never applied directly to surfaces.