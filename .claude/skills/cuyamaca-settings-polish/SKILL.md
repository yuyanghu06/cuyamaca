---
name: cuyamaca-settings-polish
description: Build the Settings view and apply final polish to Cuyamaca — model configuration UI, API key management, process health monitoring, accessibility improvements, responsive refinements, and overall UX tightening. Use this skill whenever the user wants to build the settings view, configure model providers in the UI, add API key entry, polish the app's responsiveness, improve accessibility, refine animations, add keyboard navigation, or references "phase 8", "settings", "settings view", "API key management", "model configuration", "accessibility", "polish", "keyboard navigation", or "responsive refinement". Also trigger when the user asks about WCAG compliance for glass effects, reduce transparency mode, or the settings UI for Cuyamaca. This skill assumes Phase 7 is complete (runtime agent loop, all core functionality working).
---

# Phase 8 — Settings + Polish

This skill completes the app by building the Settings view (model provider configuration, API key management, process health), polishing the UI (animations, transitions, edge cases), hardening accessibility (contrast, keyboard nav, reduced transparency), and tightening responsive behavior. After this phase, Cuyamaca is feature-complete for its initial release.

## What This Skill Produces

- Settings View with three sections: Models, Connections, About
- Model provider configuration UI: slot selection, provider picker, model picker, API key entry
- Connection settings: Ollama URL, serial port preferences
- Process health dashboard: detailed status for Ollama, arduino-cli, serial connection
- Ollama model pull/download interface
- Accessibility: reduced transparency mode, focus indicators, keyboard navigation, ARIA roles
- Animation polish: message entry, tool call pills, status transitions
- Responsive refinements: collapsible panels, mobile-width support
- Error states and empty states for all views

## Prerequisites

- Phase 7 complete: all core functionality working end-to-end
- The app has been tested through the full workflow (project → manifest → generate → flash → runtime)

## Step 1: Settings View — Models Section

Replace the Settings View placeholder with the real configuration UI.

### Layout

```
┌──────────────────────────────────────────┐
│  Settings                                │
│                                          │
│  MODELS                                  │
│  ┌────────────────────────────────────┐  │
│  │  Code Model                       │  │
│  │  ┌──────────────┐ ┌────────────┐  │  │
│  │  │Provider:     │ │Model:      │  │  │
│  │  │[Ollama    ▼] │ │[llama3.2 ▼]│  │  │
│  │  └──────────────┘ └────────────┘  │  │
│  │  API Key: [••••••••••]  [Test]    │  │
│  │  Status: ● Connected              │  │
│  └────────────────────────────────────┘  │
│  ┌────────────────────────────────────┐  │
│  │  Runtime Model                    │  │
│  │  ┌──────────────┐ ┌────────────┐  │  │
│  │  │Provider:     │ │Model:      │  │  │
│  │  │[Ollama    ▼] │ │[llava  ▼]  │  │  │
│  │  └──────────────┘ └────────────┘  │  │
│  │  ⚠ Multimodal required            │  │
│  │  API Key: [••••••••••]  [Test]    │  │
│  │  Status: ● Connected              │  │
│  └────────────────────────────────────┘  │
│                                          │
│  OLLAMA MODELS                           │
│  ┌────────────────────────────────────┐  │
│  │  Installed: llama3.2, llava        │  │
│  │  Pull new model: [model name] [↓]  │  │
│  │  Pulling llava:13b... 45%          │  │
│  └────────────────────────────────────┘  │
│                                          │
│  CONNECTIONS                             │
│  ...                                     │
└──────────────────────────────────────────┘
```

### Code Model slot

- **Provider dropdown:** Ollama, OpenAI, Anthropic, Google, Mistral
- **Model dropdown:** When Ollama is selected, populate from `listOllamaModels()`. When an external provider is selected, show a static list of supported models (from the CLAUDE.md table).
- **API Key field:** Only shown when a non-Ollama provider is selected. Masked input (`type="password"`). A "Test" button calls `checkModelHealth("code")` and shows the result.
- **Status indicator:** Green dot + "Connected" / Red dot + "Unreachable" / Amber dot + "Checking..."

### Runtime Model slot

Same as code model slot, plus:
- **Multimodal filter:** When Ollama is selected, only show multimodal models in the dropdown. When an external provider is selected, only show multimodal-capable models.
- **Warning banner:** If a text-only model is selected, show an amber warning: "This model does not support image input. Camera frames and sensor visualizations will be excluded from context."

### Ollama model management

- **Installed models list:** Show all locally available Ollama models with sizes
- **Pull interface:** Text input for model name + pull button. Shows download progress as a progress bar with percentage and speed.
- **Delete model:** Each installed model has a delete button (with confirmation)

```rust
#[tauri::command]
pub async fn pull_ollama_model(
    model: String,
    on_progress: Channel<PullProgress>,
) -> Result<(), String> {
    // POST /api/pull with streaming progress
    // Ollama streams: {"status":"pulling manifest"}, {"status":"downloading","completed":1234,"total":5678}
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum PullProgress {
    Started,
    Downloading { completed: u64, total: u64 },
    Verifying,
    Succeeded,
    Failed { error: String },
}
```

## Step 2: Settings View — Connections Section

```
┌────────────────────────────────────┐
│  CONNECTIONS                       │
│                                    │
│  Ollama                            │
│  URL: [http://localhost:11434]     │
│  Status: ● Running                 │
│  [Restart Ollama]                  │
│                                    │
│  arduino-cli                       │
│  Path: /usr/local/bin/arduino-cli  │
│  Version: 0.35.3                   │
│  Status: ● Installed               │
│  [Reinstall]                       │
│                                    │
│  Default Serial                    │
│  Baud Rate: [115200 ▼]             │
│  Auto-detect board: [✓]            │
└────────────────────────────────────┘
```

- **Ollama URL:** Editable text input. Defaults to `http://localhost:11434`. For users running Ollama on another machine, they can change this.
- **Restart Ollama:** Kills the Ollama child process and restarts it. Useful for recovery.
- **arduino-cli path:** Shows the detected or installed path. Option to reinstall if corrupted.
- **Default baud rate:** Sets the default for new projects.

## Step 3: Settings View — About Section

```
┌────────────────────────────────────┐
│  ABOUT                             │
│                                    │
│  Cuyamaca v0.1.0                   │
│  Natural language Arduino control  │
│                                    │
│  Accessibility                     │
│  [✓] Reduce transparency           │
│  [✓] Reduce motion                 │
│                                    │
│  Data                              │
│  Projects directory: ~/...         │
│  [Open in Finder/Explorer]         │
└────────────────────────────────────┘
```

## Step 4: Accessibility Hardening

### Contrast compliance

Glass effects are inherently low-contrast. Test and fix:

- All body text on glass surfaces must meet WCAG AA (4.5:1 ratio). Test against the darkest expected background.
- Secondary text (labels, timestamps) at 11-12px must meet 3:1 for large text exemption OR be bumped to 4.5:1.
- Sensor values in cyan on dark backgrounds should be tested — cyan at full opacity is usually fine, but at reduced opacity may fail.

### Reduce transparency mode

A toggle in Settings that replaces all glass backgrounds with solid dark surfaces:

```css
[data-reduce-transparency="true"] .glass-subtle,
[data-reduce-transparency="true"] .glass-standard,
[data-reduce-transparency="true"] .glass-strong {
  backdrop-filter: none;
  background: rgba(30, 30, 35, 0.95);
}
```

Also respect the OS-level preference:

```css
@media (prefers-reduced-transparency: reduce) {
  .glass-subtle, .glass-standard, .glass-strong {
    backdrop-filter: none;
    background: rgba(30, 30, 35, 0.95);
  }
}
```

### Reduce motion

Wrap all non-functional animations in the media query:

```css
@media (prefers-reduced-motion: no-preference) {
  .message-enter { animation: message-slide-in 300ms ease-out; }
  .tool-pill-pulse { animation: pill-pulse 1s ease-out; }
  .status-dot.green { animation: none; /* glow is static, not animated */ }
}
```

Functional animations (state transitions on toggles, panel collapse/expand) are kept — they're informational, not decorative.

### Keyboard navigation

Tab order follows the visual layout:
1. Sidebar navigation items
2. Main area interactive elements (inputs, buttons, component cards)
3. Parts panel / right panel elements

Specific keyboard behaviors:
- **Tab/Shift+Tab:** Move between interactive elements
- **Enter/Space:** Activate buttons, toggles, component cards
- **Escape:** Close modals, kill runtime, close expanded components
- **Arrow keys:** Navigate within dropdowns and component lists

### Focus indicators

```css
:focus-visible {
  outline: 2px solid var(--purple);
  outline-offset: 2px;
}
```

Use `:focus-visible` (not `:focus`) so mouse clicks don't show the focus ring.

### ARIA roles

- Component cards: `role="listitem"` in a `role="list"` container
- Toggle switches: `role="switch"` with `aria-checked`
- Status dots: `aria-label="Ollama: connected"` (or disconnected)
- Kill button: `aria-label="Emergency stop"` with `role="button"`
- Serial monitor: `role="log"` with `aria-live="polite"`
- Sensor state panel: `aria-live="polite"` for live updates
- Modals: `role="dialog"` with `aria-modal="true"`
- Navigation: `role="navigation"` on sidebar

## Step 5: Animation Polish

Review and refine all animations:

### Message entry (chat views)
```css
@keyframes message-slide-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
.message-enter {
  animation: message-slide-in 300ms ease-out;
}
```

### Tool call pill pulse
```css
@keyframes pill-pulse {
  0% { box-shadow: 0 0 0 0 var(--cyan-dim); }
  70% { box-shadow: 0 0 0 6px transparent; }
  100% { box-shadow: 0 0 0 0 transparent; }
}
.tool-pill-new {
  animation: pill-pulse 1s ease-out;
}
```

### Panel collapse/expand
```css
.panel-collapsible {
  transition: width 200ms ease-out;
  overflow: hidden;
}
.panel-collapsible.collapsed {
  width: 0;
}
```

### Status dot transitions
```css
.status-dot {
  transition: background-color 250ms ease, box-shadow 250ms ease;
}
```

### Input capsule focus
```css
.input-capsule:focus-within {
  border-color: rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.08);
  transition: border-color 200ms ease, background 200ms ease;
}
```

## Step 6: Error States and Empty States

Every view needs both:

### Manifest View
- **Empty:** "No project loaded. Create or open a project from the sidebar."
- **Error:** "Failed to save manifest: {error}. Check file permissions."

### Code View
- **Empty (no sketch):** "No sketch yet. Generate from manifest or upload a .ino file."
- **Empty (no project):** "Open a project to view its sketch."
- **Error (generation failed):** "Code generation failed: {error}. Check that your code model is configured in Settings."

### Chat View (code)
- **Empty (no model configured):** "Configure a code model in Settings to start chatting."
- **Error (model unreachable):** "Cannot reach {provider}. Check your connection and API key."

### Runtime Window
- **Error (serial disconnected):** "Board disconnected. Reconnect the USB cable and restart the runtime."
- **Error (model failed):** "Runtime model error: {error}. The agent loop has been stopped."
- **Error (no tools):** "No tool definitions found. Generate a sketch and approve it before starting runtime."

### Settings
- **Ollama not found:** "Ollama is not installed. [Install Now]"
- **API key invalid:** "API key test failed: 401 Unauthorized. Check your key."
- **Model pull failed:** "Failed to pull {model}: {error}. Check your internet connection."

All error states use red-tinted glass. All empty states use neutral glass with a secondary text message and a primary action button.

## Step 7: Responsive Refinements

Fine-tune the responsive behavior from Phase 1:

### ≥1100px (full layout)
- Three panels visible
- All content at full size

### 900–1099px
- Parts panel collapses to an icon-toggle in the titlebar area
- Clicking the icon slides the parts panel in as an overlay on top of the main area
- The overlay has a Glass Strong background to separate it from the content below
- Clicking outside the overlay closes it

### <900px
- Sidebar collapses to icons only (no text labels)
- Tooltip on hover shows the full label
- Parts panel hidden, accessible via titlebar toggle
- Chat input capsule remains full-width and usable

### Runtime window responsive
- Below 900px: serial monitor / sensor state / visualization stack below the chat instead of beside it
- The Kill button always remains visible regardless of window size
- On very small windows, the Kill button anchors to the bottom-right corner

## Step 8: Final Polish Items

### Loading states
- App startup: show a minimal splash with the Cuyamaca name while Ollama and arduino-cli are detected
- Model slot configuration: show a spinner while the health check runs
- Project loading: show skeleton cards while manifest is read from disk

### Tooltips
- Status dots: tooltip showing detailed status ("Ollama: running on localhost:11434")
- Component type icons: tooltip showing the full type name
- Abbreviated sensor values: tooltip showing the full precision value

### Notifications
- Flash success: brief green notification toast "Sketch flashed successfully"
- Serial disconnect: amber notification toast "Board disconnected"
- Model error: red notification toast with the error message

Use a simple toast system — notifications appear at the top-right, auto-dismiss after 4 seconds, stackable.

### Window close handlers
- Project window close: save any unsaved manifest changes
- Runtime window close: trigger kill (CMD:stop + close serial)
- App quit: close all windows, stop all child processes

## Step 9: Verify

### Settings verification
1. Navigate to Settings view
2. Configure the code model: select a provider, select a model, enter an API key
3. Click "Test" — status shows green/red
4. Configure the runtime model: only multimodal models appear in the list
5. Pull a new Ollama model — progress bar shows download progress
6. Change the Ollama URL — health check updates

### Accessibility verification
1. Enable "Reduce transparency" — all glass surfaces become solid dark
2. Tab through the entire app — focus ring appears on every interactive element
3. Operate all controls with keyboard only (no mouse)
4. Run a screen reader (VoiceOver on macOS, Narrator on Windows) — all elements are announced correctly
5. Check contrast ratios with browser dev tools — all text passes WCAG AA

### Responsive verification
1. Resize window to 1000px wide — parts panel collapses to icon toggle
2. Click the toggle — parts panel slides in as overlay
3. Resize to 800px — sidebar collapses to icons, parts panel hidden
4. Chat input remains usable at all widths
5. Runtime window responds appropriately to narrow widths

### Error state verification
1. Disconnect Ollama — status dot goes red, appropriate error messages appear
2. Disconnect the Arduino board — serial disconnect notification appears
3. Enter an invalid API key — "Test" shows unauthorized error
4. Try to generate code with no model configured — error message with link to Settings

## What NOT to Do

- Do not add a light mode. The dark industrial theme is load-bearing for the glass hierarchy.
- Do not add custom window chrome or frameless windows. Native title bars work. Custom chrome adds complexity for minimal benefit.
- Do not add onboarding tutorials or guided tours. The app is for technical users (Arduino hobbyists) who can figure out a three-panel layout.
- Do not add analytics, telemetry, or crash reporting. Everything is local.
- Do not add cloud sync, user accounts, or sharing features. Local-only.
- Do not add auto-update. The user downloads new versions manually. Auto-update infrastructure is a separate concern.
- Do not over-animate. The dark industrial aesthetic calls for precision, not playfulness. Keep animations minimal and purposeful.