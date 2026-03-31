import { useState, useRef, useEffect, useCallback } from "react";
import { sendChatMessage, clearChatHistory } from "../commands/codegen";
import type {
  Project,
  GeneratedSketchResponse,
  ChatResponse,
} from "../types/manifest";
import GlassPanel from "../components/GlassPanel";

interface ChatViewProps {
  project: Project | null;
  onPendingSketch: (sketch: GeneratedSketchResponse | null) => void;
  onSwitchToCode: () => void;
}

interface ChatBubble {
  role: "user" | "assistant";
  text: string;
  hasSketch?: boolean;
}

export default function ChatView({
  project,
  onPendingSketch,
  onSwitchToCode,
}: ChatViewProps) {
  const [messages, setMessages] = useState<ChatBubble[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll on new messages
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, loading]);

  // Clear chat when project changes
  useEffect(() => {
    setMessages([]);
    clearChatHistory().catch(() => {});
  }, [project?.name]);

  const handleSend = useCallback(async () => {
    const msg = input.trim();
    if (!msg || loading) return;

    setInput("");
    setMessages((prev) => [...prev, { role: "user", text: msg }]);
    setLoading(true);

    try {
      const response: ChatResponse = await sendChatMessage(msg);

      const bubble: ChatBubble = {
        role: "assistant",
        text: response.text,
        hasSketch: !!response.sketch,
      };
      setMessages((prev) => [...prev, bubble]);

      if (response.sketch) {
        onPendingSketch(response.sketch);
      }
    } catch (err) {
      setMessages((prev) => [
        ...prev,
        { role: "assistant", text: `Error: ${err}` },
      ]);
    } finally {
      setLoading(false);
      inputRef.current?.focus();
    }
  }, [input, loading, onPendingSketch]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  if (!project) {
    return (
      <div className="view-placeholder">
        <div className="view-placeholder-title">Chat</div>
        <div className="view-placeholder-subtitle">
          Open a project to chat with the code model.
        </div>
      </div>
    );
  }

  return (
    <div className="chat-view">
      <div className="chat-messages" ref={scrollRef}>
        {messages.length === 0 && (
          <div className="chat-empty">
            <div className="chat-empty-title">Code Model Chat</div>
            <div className="chat-empty-desc">
              Ask the code model to generate, modify, or explain your Arduino
              sketch. Changes appear as diffs in the Code view.
            </div>
          </div>
        )}

        {messages.map((msg, idx) => (
          <div
            key={idx}
            className={`chat-bubble ${msg.role === "user" ? "chat-bubble-user" : "chat-bubble-ai"}`}
          >
            <GlassPanel
              tier={msg.role === "user" ? "strong" : "standard"}
              className={`chat-bubble-inner ${msg.role === "user" ? "chat-user-glass" : "chat-ai-glass"}`}
            >
              <div className="chat-bubble-text">{msg.text}</div>
              {msg.hasSketch && (
                <button
                  className="chat-review-btn"
                  onClick={onSwitchToCode}
                >
                  Review in Code view →
                </button>
              )}
            </GlassPanel>
          </div>
        ))}

        {loading && (
          <div className="chat-bubble chat-bubble-ai">
            <GlassPanel tier="standard" className="chat-bubble-inner chat-ai-glass">
              <div className="chat-loading">
                <span className="chat-loading-dot" />
                <span className="chat-loading-dot" />
                <span className="chat-loading-dot" />
              </div>
            </GlassPanel>
          </div>
        )}
      </div>

      <div className="chat-input-area">
        <GlassPanel tier="standard" className="chat-input-capsule">
          <textarea
            ref={inputRef}
            className="chat-input"
            placeholder="Describe what to change…"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={1}
            disabled={loading}
          />
          <button
            className="chat-send-btn"
            onClick={handleSend}
            disabled={loading || !input.trim()}
          >
            ↑
          </button>
        </GlassPanel>
      </div>
    </div>
  );
}
