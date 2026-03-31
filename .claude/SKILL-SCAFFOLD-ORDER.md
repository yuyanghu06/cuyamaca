Phase 1 — Scaffold (364 lines): Tauri v2 project, dark industrial theme, three-panel layout, IPC verification. Same vertical slice pattern as Sierra but with the oscilloscope-inspired palette.

Phase 2 — LLM Abstraction (444 lines): The ModelProvider trait supporting Ollama + four external APIs, two independent model slots (code + runtime), API key storage in OS keychain, health checks.

Phase 3 — Project + Manifest (428 lines): Project CRUD and file system, manifest data model with the full component library, parts editor UI with component picker and pin assignment, project list in sidebar.

Phase 4 — Code Generation (419 lines): Code model generates sketches from manifests, diff computation and display in the Code View, approve/reject workflow, tool synthesis (sketch → tools.json), sketch upload support, code chat for conversational modifications.

Phase 5 — Arduino Flash (401 lines): arduino-cli detection/install, board detection, core management, compile + upload pipeline, flash progress UI, pre-flash validation.

Phase 6 — Serial + Sensors (526 lines): Serial connection with concurrent read/write, structured output parser (SENSOR_ID:VALUE), sensor state store, sensor visualization renderer (PNGs from spatial data), camera frame capture, serial monitor + sensor state + viz frontend components.

Phase 7 — Runtime Agent (613 lines): The main event — runtime window, multimodal context assembly (text + sensor viz + camera), tool call dispatch via serial commands, the agentic observe-decide-act loop, chat with tool call pills, the Kill button with Escape shortcut.

Phase 8 — Settings + Polish (416 lines): Settings view (model config, API keys, Ollama model pull, connections), accessibility (reduced transparency, focus indicators, ARIA, keyboard nav), responsive refinements, error/empty states, animation polish, toast notifications.

Phase 9 - README 

Phase 10 - Installer + wizard