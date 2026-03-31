import { useRef, useEffect } from "react";

interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "tool-started" | "tool-completed";
  content: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  toolSuccess?: boolean;
}

interface RuntimeChatProps {
  messages: ChatMessage[];
  input: string;
  running: boolean;
  onInputChange: (value: string) => void;
  onSend: () => void;
  onKill: () => void;
}

export default function RuntimeChat({
  messages,
  input,
  running,
  onInputChange,
  onSend,
  onKill,
}: RuntimeChatProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey && !running) {
      e.preventDefault();
      onSend();
    }
  };

  const formatArgs = (args?: Record<string, unknown>): string => {
    if (!args || Object.keys(args).length === 0) return "";
    return Object.entries(args)
      .map(([k, v]) => `${k}=${v}`)
      .join(", ");
  };

  return (
    <div className="runtime-chat">
      <div className="runtime-chat-messages" ref={scrollRef}>
        {messages.length === 0 && (
          <div className="runtime-chat-empty">
            <p>Connected to your board. Tell the robot what to do.</p>
          </div>
        )}
        {messages.map((msg) => {
          if (msg.role === "user") {
            return (
              <div key={msg.id} className="runtime-msg runtime-msg-user">
                <div className="runtime-msg-bubble user">{msg.content}</div>
              </div>
            );
          }
          if (msg.role === "assistant") {
            return (
              <div key={msg.id} className="runtime-msg runtime-msg-assistant">
                <div className="runtime-msg-bubble assistant">{msg.content}</div>
              </div>
            );
          }
          if (msg.role === "tool-started") {
            return (
              <div key={msg.id} className="runtime-msg runtime-msg-tool">
                <div className="tool-call-pill pending">
                  <span className="tool-call-dot">◉</span>
                  <span className="tool-call-name">{msg.toolName}</span>
                  {msg.toolArgs && Object.keys(msg.toolArgs).length > 0 && (
                    <span className="tool-call-args">
                      {formatArgs(msg.toolArgs)}
                    </span>
                  )}
                </div>
              </div>
            );
          }
          if (msg.role === "tool-completed") {
            return (
              <div key={msg.id} className="runtime-msg runtime-msg-tool">
                <div
                  className={`tool-call-pill ${msg.toolSuccess ? "success" : "error"}`}
                >
                  <span className="tool-call-dot">
                    {msg.toolSuccess ? "✓" : "✗"}
                  </span>
                  <span className="tool-call-name">{msg.toolName}</span>
                  <span className="tool-call-output">{msg.content}</span>
                </div>
              </div>
            );
          }
          return null;
        })}
        {running && (
          <div className="runtime-msg runtime-msg-assistant">
            <div className="runtime-thinking">
              <span className="thinking-dot" />
              <span className="thinking-dot" />
              <span className="thinking-dot" />
            </div>
          </div>
        )}
      </div>

      <div className={`runtime-input-bar ${running ? "thinking" : ""}`}>
        <input
          type="text"
          className="runtime-input"
          placeholder="Tell the robot what to do..."
          value={input}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={running}
        />
        <button
          className="runtime-send-btn"
          onClick={onSend}
          disabled={running || !input.trim()}
        >
          Send
        </button>
        <button className="runtime-kill-btn" onClick={onKill} aria-label="Emergency stop">
          KILL
        </button>
      </div>
    </div>
  );
}
