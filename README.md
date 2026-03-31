# Cuyamaca

> natural language control for Arduino-based robotics. define your hardware, generate or upload sketches, and talk to your board through a local LLM.

<!-- screenshot: main project window showing the three-panel layout with parts editor, code view, and sidebar -->

## what it does

Cuyamaca is a desktop app that lets you control Arduino boards using plain language. You describe your hardware in a visual parts editor, specifying what's connected and where. The app uses a code model to generate an Arduino sketch from that description, shows you the code for review, and flashes it to your board with one click.

After flashing, a runtime window opens where you type natural language commands like "move forward slowly" or "scan for obstacles." A runtime model reads live sensor data and camera feeds from your board, decides what actions to take, and sends serial commands automatically. The whole loop runs until you hit the kill button.

Everything runs locally by default using [Ollama](https://ollama.com) for inference. No cloud accounts, no API keys, no internet connection required. If you want to use external model APIs (OpenAI, Anthropic, Google, Mistral), those are available as optional configuration.

## how it works

Cuyamaca uses two independent model slots. The **code model** handles sketch generation and modification during project setup. The **runtime model** controls your board live after flashing, reading sensor data and making decisions in real time. Each slot is configured separately, so you can use a cloud API for code generation and a local model for runtime control, or any combination that works for you.

Everything is driven by the **manifest**, a hardware definition you build in the parts editor. It lists your board type, serial port, and every connected component with its pin assignments. The code model reads this manifest when generating sketches, so it always knows exactly what hardware is available. You never have to explain your wiring in a chat message.

Generated sketches follow a **structured serial protocol** where all sensor output uses the format `SENSOR_ID:VALUE`. This isn't a suggestion; the code model enforces it. Because the output is structured, the runtime model can reliably parse sensor readings, and the app can render live sensor state and visualizations during the control loop.

You don't need a terminal after installation. The app manages arduino-cli (for compiling and flashing), Ollama (for local inference), serial connections, and model configuration internally. The only external interaction is plugging in your board.

## requirements

**Operating system.** macOS or Windows. Linux is not currently targeted.

**Hardware.** An Arduino-compatible board (Uno, Mega, Nano, ESP32, or similar) connected via USB. For vision features, an ESP32-CAM connected over WiFi is supported but not required.

**Memory.** If you're running models locally with Ollama, plan for at least 8GB of RAM for smaller models and 16GB or more for capable ones. If you only use external APIs for both model slots, memory requirements are minimal.

**Ollama and arduino-cli.** Both are required, but the app installs them automatically on first launch. You don't need to set them up beforehand.

## installation

1. Download the installer for your platform from [GitHub Releases](https://github.com/user/cuyamaca/releases) (`.dmg` for macOS, `.exe` or `.msi` for Windows).
2. Run the installer.

On first launch, the app checks whether Ollama and arduino-cli are present on your system. If either is missing, it downloads and installs them for you. This happens once and takes a few minutes depending on your internet connection. You won't need to open a terminal at any point.

<!-- note: update the GitHub Releases link once the distribution channel is finalized -->

## getting started

1. **Connect your board.** Plug your Arduino into a USB port. The app detects available serial ports automatically.

2. **Create a project.** Click the + button in the sidebar. Give your project a name, select your board type (e.g., `arduino:avr:uno`), and choose the serial port your board is connected to.

3. **Add components.** Open the parts editor and define what's connected to your board: motors, sensors, servos, LEDs, cameras. For each component, assign the correct pins. The app knows the pin configuration for every supported component type.

4. **Generate a sketch.** Switch to the chat view and ask the code model to generate a sketch from your manifest. It will produce an Arduino `.ino` file that sets up all your components, reads sensors at regular intervals, and includes a serial command dispatcher. Review the code in the code view. If you want changes, ask for them in chat and review the diff.

5. **Flash.** When you're satisfied with the sketch, click "Approve & Flash." The app compiles the sketch using arduino-cli and uploads it to your board. This takes a few seconds.

6. **Talk to your board.** The runtime window opens after a successful flash. Type commands in natural language:

   ```
   move forward at half speed
   scan left and right for obstacles
   what's the temperature?
   turn the LED on
   stop
   ```

   The runtime model reads sensor data from your board, decides which serial commands to send, and executes them. You can see raw serial output, parsed sensor state, and sensor visualizations updating in real time on the right panel.

## model configuration

Both model slots are configured in the Settings view, accessible from the sidebar.

**Code model.** This handles sketch generation, modification, and tool synthesis. Any text-capable model works. You can use a local Ollama model (any model you have installed) or an external API: OpenAI (GPT-4o, o3), Anthropic (Claude Sonnet, Claude Opus), Google (Gemini 1.5 Pro, Gemini 2.0 Flash), or Mistral (Codestral). Stronger coding models produce better sketches.

**Runtime model.** This controls your board live during the agentic loop. If you have a camera or spatial sensors (line arrays, LIDAR, IMU), the runtime model receives sensor visualization images and camera frames as part of its context, so it must be multimodal. Supported local models include LLaVA 1.6, Llama 3.2 Vision, BakLLaVA, and Moondream 2. External multimodal options include GPT-4o, Claude Sonnet/Opus, and Gemini. If you select a text-only model for the runtime slot, the app will warn you that image context will be dropped.

API keys for external providers are stored in your operating system's keychain, not in config files. You enter them once in Settings and the app handles the rest.

## supported hardware

| Category | Components |
|---|---|
| **Actuators** | DC motors, servos, stepper motors, relays, LEDs |
| **Distance / proximity** | Ultrasonic (HC-SR04), IR distance, LIDAR (TF-Mini) |
| **Motion / orientation** | IMU (MPU-6050), magnetometer (HMC5883L), rotary encoders |
| **Touch / tactile** | Bump switches, line sensor arrays, force sensors |
| **Environmental** | Temperature/humidity (DHT22), barometer (BMP280), light sensor (BH1750), gas sensors (MQ series) |
| **Vision** | ESP32-CAM (WiFi, JPEG streaming) |

Each component type has a known pin configuration and a known serial output format. When the code model generates a sketch, it uses this information to write correct setup, reading, and command dispatch code for every component in your manifest.

## running Ollama and arduino-cli manually

If you already have Ollama or arduino-cli installed, or prefer to manage them yourself, the app can use your existing installations.

### Ollama

Install Ollama from [ollama.com](https://ollama.com). Start the server:

```bash
ollama serve
```

It runs on `http://localhost:11434` by default. Pull a model before using it:

```bash
ollama pull llama3.2
ollama pull llava        # multimodal, good for runtime
```

In Cuyamaca's Settings, the Ollama URL defaults to `http://localhost:11434`. If you're running Ollama on a different machine or port, change the URL there.

### arduino-cli

Install arduino-cli from [arduino.github.io/arduino-cli](https://arduino.github.io/arduino-cli/latest/installation/). Install the core for your board:

```bash
arduino-cli core install arduino:avr       # Uno, Mega, Nano
arduino-cli core install esp32:esp32       # ESP32 boards
```

On macOS, arduino-cli is typically installed to `/usr/local/bin/arduino-cli` or via Homebrew. On Windows, the default path depends on your installation method. The app auto-detects the path, but you can verify it in Settings under Connections.

If you let the app manage these tools as child processes (the default), you don't need to do any of this. These instructions are for users who want more control over their toolchain.

## example projects

### obstacle-avoiding robot

Two DC motors and one front-facing ultrasonic sensor on an Arduino Uno. Define the motors and sensor in the parts editor, generate a sketch, and flash. In the runtime window, tell the robot to "explore the room and avoid walls." The runtime model reads the distance sensor, drives forward when the path is clear, and turns when something is close.

### environmental monitor

A DHT22 temperature/humidity sensor, a BH1750 light sensor, and a BMP280 barometer, all connected over I2C to an Arduino Nano. After flashing, ask questions in the runtime window: "what's the temperature right now?", "is it getting darker?", "what's the barometric pressure?" The runtime model reads the sensors and responds conversationally.

### camera-guided rover

Two DC motors, a front ultrasonic sensor, and an ESP32-CAM streaming JPEG over WiFi. This project uses a multimodal runtime model (like LLaVA or GPT-4o) that can see through the camera. Tell the rover to "go toward the red object" or "describe what you see." The model combines visual input with distance readings to navigate.

## troubleshooting

**Board not detected.** Make sure your USB cable supports data transfer (some cables are charge-only). On Windows, you may need to install USB-to-serial drivers for your board. Check that the correct serial port is selected in your project's manifest.

**Compile or flash failures.** Verify that you selected the right board type in the manifest (e.g., `arduino:avr:uno` for an Uno). If the sketch uses external libraries, they may need to be installed through arduino-cli. Check the error output in the code view for specifics.

**Ollama not connecting.** Check the sidebar status indicator. If it's red, Ollama isn't running. Try restarting it from Settings, or run `ollama serve` in a terminal to see if there are errors.

**Garbled serial output.** This usually means the baud rate in your manifest doesn't match the baud rate in the sketch. The default is 115200. Make sure both match.

**Runtime model ignores sensors or camera.** If you're using a text-only model in the runtime slot, it can't process sensor visualization images or camera frames. Switch to a multimodal model (LLaVA, GPT-4o, Claude, Gemini) in Settings.

**Flash fails on macOS with permission errors.** Your user account may not have access to the serial port. Try disconnecting and reconnecting the board, or check System Settings for any security prompts about allowing USB access.

## license

See [LICENSE](LICENSE) for details.
