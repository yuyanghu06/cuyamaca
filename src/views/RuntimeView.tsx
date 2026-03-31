import { useState, useEffect, useCallback, useRef } from "react";
import { runtimeSendMessage, runtimeKill } from "../commands/runtime";
import { subscribeSerial } from "../commands/serial";
import SerialMonitor from "../components/SerialMonitor";
import SensorStatePanel from "../components/SensorStatePanel";
import SensorVizPanel from "../components/SensorVizPanel";
import RuntimeChat from "../components/RuntimeChat";
import type {
  AgentEvent,
  SerialEvent,
  SensorSnapshot,
} from "../types/manifest";

interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "tool-started" | "tool-completed";
  content: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  toolSuccess?: boolean;
}

export default function RuntimeView() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [running, setRunning] = useState(false);
  const [connected, setConnected] = useState(true);
  const [serialLines, setSerialLines] = useState<string[]>([]);
  const [sensors, setSensors] = useState<SensorSnapshot[]>([]);
  const msgIdRef = useRef(0);

  const nextId = () => String(++msgIdRef.current);

  // Subscribe to serial events on mount
  useEffect(() => {
    let cancelled = false;
    subscribeSerial((event: SerialEvent) => {
      if (cancelled) return;
      if (event.event === "rawLine") {
        setSerialLines((prev) => {
          const next = [...prev, event.data];
          return next.length > 2000 ? next.slice(-1500) : next;
        });
      } else if (event.event === "sensorUpdate") {
        setSensors((prev) => {
          const idx = prev.findIndex(
            (s) => s.sensor_id === event.data.sensor_id,
          );
          const snap: SensorSnapshot = {
            sensor_id: event.data.sensor_id,
            label: event.data.sensor_id,
            component_type: "",
            values: event.data.values,
            formatted: event.data.formatted,
            timestamp_ms: Date.now(),
          };
          if (idx >= 0) {
            const copy = [...prev];
            copy[idx] = snap;
            return copy;
          }
          return [...prev, snap];
        });
      } else if (event.event === "disconnected") {
        setConnected(false);
      }
    }).catch(() => {
      /* serial not open yet */
    });

    return () => {
      cancelled = true;
    };
  }, []);

  // Escape key = kill
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleKill();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const handleKill = useCallback(async () => {
    try {
      await runtimeKill();
    } catch {
      /* best effort */
    }
    setRunning(false);
    setMessages((prev) => [
      ...prev,
      {
        id: nextId(),
        role: "assistant",
        content: "⛔ Emergency stop — all motors halted.",
      },
    ]);
  }, []);

  const handleSend = useCallback(async () => {
    const msg = input.trim();
    if (!msg || running) return;
    setInput("");
    setRunning(true);

    setMessages((prev) => [
      ...prev,
      { id: nextId(), role: "user", content: msg },
    ]);

    try {
      await runtimeSendMessage(msg, (event: AgentEvent) => {
        switch (event.event) {
          case "modelResponse":
            setMessages((prev) => [
              ...prev,
              { id: nextId(), role: "assistant", content: event.data },
            ]);
            break;
          case "toolCallStarted":
            setMessages((prev) => [
              ...prev,
              {
                id: nextId(),
                role: "tool-started",
                content: `Calling ${event.data.tool_name}`,
                toolName: event.data.tool_name,
                toolArgs: event.data.arguments,
              },
            ]);
            break;
          case "toolCallCompleted":
            setMessages((prev) => [
              ...prev,
              {
                id: nextId(),
                role: "tool-completed",
                content: event.data.output,
                toolName: event.data.tool_name,
                toolSuccess: event.data.success,
              },
            ]);
            break;
          case "turnComplete":
            setRunning(false);
            break;
          case "sessionEnded":
            setRunning(false);
            setMessages((prev) => [
              ...prev,
              {
                id: nextId(),
                role: "assistant",
                content: "Session ended by the model.",
              },
            ]);
            break;
          case "error":
            setRunning(false);
            setMessages((prev) => [
              ...prev,
              {
                id: nextId(),
                role: "assistant",
                content: `Error: ${event.data}`,
              },
            ]);
            break;
        }
      });
    } catch (err) {
      setRunning(false);
      setMessages((prev) => [
        ...prev,
        {
          id: nextId(),
          role: "assistant",
          content: `Error: ${err}`,
        },
      ]);
    }
  }, [input, running]);

  return (
    <div className="runtime-window">
      <div className="runtime-left">
        <RuntimeChat
          messages={messages}
          input={input}
          running={running}
          onInputChange={setInput}
          onSend={handleSend}
          onKill={handleKill}
        />
      </div>
      <div className="runtime-right">
        <div className="runtime-panel runtime-serial">
          <SerialMonitor lines={serialLines} connected={connected} />
        </div>
        <div className="runtime-panel runtime-sensors">
          <SensorStatePanel sensors={sensors} />
        </div>
        <div className="runtime-panel runtime-viz">
          <SensorVizPanel connected={connected} />
        </div>
      </div>
    </div>
  );
}
