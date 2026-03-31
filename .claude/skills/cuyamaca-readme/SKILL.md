---
name: cuyamaca-readme
description: Write the user-facing README.md for the Cuyamaca Tauri v2 desktop app (natural language Arduino/robotics control). Use this skill whenever the user asks to create, write, draft, or update the README for Cuyamaca, or mentions "readme", "documentation", "project description", "repo docs", or "GitHub page" in the context of Cuyamaca. Also trigger when the user asks about what to put in the README, how to describe the project publicly, installation instructions for end users, or setup guides. This skill produces a polished, user-facing README — not developer docs or architecture specs.
---

# Cuyamaca README Skill

This skill produces the `README.md` for the Cuyamaca repository. The audience is someone who just found the repo or downloaded the app — not a contributor or developer. The tone is clear, warm, and confident. The writing style is lowercase prose for public-facing text; no em dashes. Sentences should be direct without being terse.

Read the project's `CLAUDE.md` before writing. The README must be consistent with the architecture, supported platforms, and design decisions documented there. If anything in CLAUDE.md conflicts with this skill, CLAUDE.md wins — it is the authoritative spec.

## Voice and Style

- Write in second person ("you") addressing the user who just downloaded the app
- Keep paragraphs short — 2-3 sentences max
- No marketing fluff, no superlatives, no "revolutionary" or "cutting-edge"
- Technical terms are fine when they're the right word (arduino-cli, serial, baud rate) but explain them briefly on first use if a hobbyist might not know them
- Use lowercase for headings that aren't proper nouns (e.g., "what it does", "getting started", not "What It Does")
- No em dashes. Use commas, periods, or parentheses instead
- Code blocks for anything the user types or sees in a terminal

## README Structure

Follow this exact section order. Every section is required.

### 1. Project Title and Tagline

The title is just `Cuyamaca`. Below it, one sentence explaining what the app does in plain language:

> natural language control for arduino-based robotics. define your hardware, generate or upload sketches, and talk to your board through a local llm.

Render this as a blockquote or subtitle, not a heading. Keep it lowercase.

### 2. what it does

One paragraph (3-5 sentences) covering the core loop:
- You describe your hardware in a visual parts editor (the manifest)
- The app generates Arduino sketches from your description using a code model
- You review the code, approve it, and the app flashes it to your board
- After flashing, you control the board through natural language in a chat interface
- A runtime model reads sensor data and camera feeds, decides what to do, and sends commands over serial

Emphasize: runs locally by default. No cloud accounts required. External model APIs (OpenAI, Anthropic, etc.) are optional.

### 3. how it works

Brief architectural overview for curious users. Not a developer doc, but enough to understand the pieces:

- **Two model roles**: code model generates sketches, runtime model controls the board live. Each is configured independently. You can use a cloud API for code generation and a local model for runtime, or any combination.
- **Manifest-driven**: the parts editor creates a hardware manifest (board type, components, pin assignments). All code generation is grounded in this manifest.
- **Structured serial**: the app enforces a structured output protocol (`SENSOR_ID:VALUE`) in generated sketches so sensor data is machine-readable, not debug strings.
- **Everything through the desktop app**: no terminal commands after installation. The app manages arduino-cli, Ollama, serial connections, and model configuration internally.

Keep this section to 4-6 short paragraphs or a brief paragraph followed by a concise description list. Not bullet soup.

### 4. requirements

What the user needs before installing:

- **Operating system**: macOS or Windows. Linux is not targeted.
- **Hardware**: an Arduino-compatible board (Uno, Mega, Nano, ESP32, etc.) connected via USB. Optionally an ESP32-CAM for vision features (connects over WiFi).
- **RAM**: enough to run a local LLM if using Ollama. 8GB minimum for small models, 16GB+ recommended for capable models. If using only external APIs, RAM requirements are minimal.
- **Ollama and arduino-cli**: required, but the app installs them automatically on first launch. No manual setup needed.

Format as a short paragraph per item, not a raw bullet list.

### 5. installation

Two steps:
1. Download the installer for your platform (`.dmg` for macOS, `.exe`/`.msi` for Windows)
2. Run it

Then describe what happens on first launch: the app checks for Ollama and arduino-cli, downloads and installs them if missing, and walks you through initial setup. The user never opens a terminal.

Mention where to get the installer (GitHub Releases page, or whatever the distribution channel is). If the distribution channel isn't finalized yet, use `[GitHub Releases](link)` as a placeholder and note it.

### 6. getting started

Walk through the first-use flow:

1. **Connect your board** — plug in your Arduino via USB. The app detects available serial ports.
2. **Create a project** — give it a name, select your board type, and choose the serial port.
3. **Add components** — use the parts editor to define what's connected: motors, sensors, servos, LEDs, etc. Assign pins for each.
4. **Generate a sketch** — the code model reads your manifest and writes an Arduino sketch. Review the code, see the diff if it's a modification, and approve it.
5. **Flash** — one click. The app invokes arduino-cli to compile and upload.
6. **Talk to your board** — the runtime window opens. Type natural language commands ("move forward slowly", "scan for obstacles", "turn the LED on"). The runtime model reads sensor data, decides which commands to send, and executes them.

Use numbered steps with brief explanations. Include example prompts in the runtime step.

### 7. model configuration

Explain the two model slots:

**Code model**: handles sketch generation and modification. Any model works (text-only is fine). Supports Ollama (local) and external APIs (OpenAI, Anthropic, Google, Mistral). Best results come from strong coding models.

**Runtime model**: controls the board live during the agentic loop. Must be multimodal if you have a camera or spatial sensors, because it receives sensor visualization images and camera frames as context. Supports Ollama multimodal models (LLaVA, Llama 3.2 Vision, Moondream 2) and external multimodal APIs (GPT-4o, Claude Sonnet/Opus, Gemini).

Explain that both slots are configured in Settings, and that API keys for external providers are stored in the OS keychain.

### 8. supported hardware

List the component types the app understands, grouped by category. Use a table or compact description list:

**Actuators**: DC motors, servos, stepper motors, relays, LEDs
**Distance/proximity**: ultrasonic (HC-SR04), IR distance, LIDAR (TF-Mini)
**Motion/orientation**: IMU (MPU-6050), magnetometer, rotary encoders
**Touch/tactile**: bump switches, line sensor arrays, force sensors
**Environmental**: temperature/humidity (DHT22), barometer (BMP280), light (BH1750), gas (MQ series)
**Vision**: ESP32-CAM (WiFi, JPEG streaming)

Note that each component type has a known pin configuration and serial output format. The code model uses this to generate correct setup and reading code.

### 9. running ollama and arduino-cli manually

For users who already have these tools or want to manage them outside the app:

- **Ollama**: how to install standalone, `ollama serve`, default port (11434), pulling a model (`ollama pull llama3.2`). macOS vs Windows differences if any.
- **arduino-cli**: how to install standalone, how to install board cores (`arduino-cli core install arduino:avr`), default paths. macOS vs Windows differences.
- How to point the app at externally-running instances instead of letting it manage them as child processes.

### 10. example projects

Provide 2-3 brief project sketches (not full tutorials, just "here's what you could build"):

1. **Obstacle-avoiding robot** — two DC motors, one ultrasonic sensor. The runtime model drives forward, checks distance, turns when something is close.
2. **Environmental monitor** — DHT22 + BH1750 + BMP280. The runtime model reads conditions and responds to questions ("what's the temperature?", "is it getting darker?").
3. **Camera-guided rover** — two motors, ultrasonic, ESP32-CAM. The multimodal runtime model sees through the camera and navigates based on what it sees.

These should be aspirational but achievable. 3-4 sentences each.

### 11. troubleshooting

Cover the most common issues:

- Board not detected (check USB cable, drivers on Windows, serial port selection)
- arduino-cli compile failures (wrong board type selected, missing libraries)
- Ollama connection errors (is it running? check the status indicator in the sidebar)
- Serial communication garbled (baud rate mismatch between manifest and sketch)
- Runtime model not responding to sensor data (is the model multimodal? text-only models can't read sensor visualizations)
- Flash fails on macOS (serial port permissions)

Brief, actionable fixes. 1-2 sentences per issue.

### 12. license

Placeholder for the project's license. Use `[LICENSE](LICENSE)` link.

## Formatting Rules

- Use `#` for the project title only. All sections use `##`. Subsections use `###`.
- No table of contents (the README isn't long enough to need one)
- No badges unless the user specifically requests them
- No screenshots or images unless the user provides them (note where they would go with `<!-- screenshot: description -->` comments)
- Keep the total README under 500 lines. If it's getting longer, cut prose, not sections.
- Headings are lowercase except proper nouns (Cuyamaca, Arduino, Ollama, Home Assistant, etc.)
- Use fenced code blocks (triple backtick) for terminal commands, serial output examples, and file paths

## What NOT to Do

- Do not write developer documentation (contributing guide, architecture overview, build-from-source instructions). This is a user-facing README.
- Do not include the full manifest JSON schema. Show one brief example if needed, but point users to docs for the full spec.
- Do not list every supported model with version numbers. They change constantly. Give categories and examples.
- Do not promise features from the future roadmap (CAD integration, multi-board, Python serial scripts) as current capabilities.
- Do not describe the UI design system, glass hierarchy, or CSS values. The user sees the app, they don't need to know how it's styled.
- Do not use em dashes anywhere in the document.
- Do not include a "built with" or "tech stack" section. Users don't care that the backend is Rust.