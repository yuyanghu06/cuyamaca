import { useState, useEffect, useRef, useCallback } from "react";

interface SerialMonitorProps {
  lines: string[];
  connected: boolean;
}

const MAX_VISIBLE = 1000;

// Lines matching SENSOR_ID:VALUE are dimmed (already parsed in sensor state panel)
const STRUCTURED_RE = /^[A-Z][A-Z0-9_]*:.+$/;

export default function SerialMonitor({ lines, connected }: SerialMonitorProps) {
  const [paused, setPaused] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  const displayLines = lines.slice(-MAX_VISIBLE);

  // Auto-scroll to bottom when not paused
  useEffect(() => {
    if (!paused && bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: "auto" });
    }
  }, [displayLines.length, paused]);

  const handleScroll = useCallback(() => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    const atBottom = scrollHeight - scrollTop - clientHeight < 40;
    if (!atBottom && !paused) {
      setPaused(true);
    }
  }, [paused]);

  return (
    <div className="serial-monitor">
      <div className="serial-monitor-header">
        <span className="label">Serial Monitor</span>
        <div className="serial-monitor-controls">
          <span
            className={`serial-status-dot ${connected ? "connected" : "disconnected"}`}
          />
          <button
            className="serial-pause-btn"
            onClick={() => setPaused((p) => !p)}
            title={paused ? "Resume auto-scroll" : "Pause auto-scroll"}
          >
            {paused ? "▶" : "⏸"}
          </button>
        </div>
      </div>
      <div
        className="serial-monitor-output"
        ref={scrollRef}
        onScroll={handleScroll}
      >
        {displayLines.map((line, i) => {
          const isStructured = STRUCTURED_RE.test(line);
          const isError = line.startsWith("ERROR:");
          return (
            <div
              key={i}
              className={`serial-line ${isStructured ? "structured" : ""} ${isError ? "error" : ""}`}
            >
              {line}
            </div>
          );
        })}
        <div ref={bottomRef} />
      </div>
      {paused && (
        <button
          className="serial-resume-bar"
          onClick={() => setPaused(false)}
        >
          ↓ Resume auto-scroll
        </button>
      )}
    </div>
  );
}
