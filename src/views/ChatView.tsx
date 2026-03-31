import { useState } from "react";
import { ping } from "../commands";
import GlassPanel from "../components/GlassPanel";

export default function ChatView() {
  const [ipcResult, setIpcResult] = useState<string | null>(null);
  const [ipcLoading, setIpcLoading] = useState(false);

  async function testIpc() {
    setIpcLoading(true);
    try {
      const result = await ping("hello from frontend");
      setIpcResult(result);
    } catch (err) {
      setIpcResult(`Error: ${err}`);
    } finally {
      setIpcLoading(false);
    }
  }

  return (
    <div className="view-placeholder">
      <div className="view-placeholder-title">Chat</div>
      <div className="view-placeholder-subtitle">
        Converse with the code model to generate, modify, and understand your Arduino sketches.
        Changes are shown as diffs and require your approval before flashing.
      </div>

      <GlassPanel tier="standard" className="ipc-test">
        <button onClick={testIpc} disabled={ipcLoading}>
          {ipcLoading ? "…" : "Test IPC Bridge"}
        </button>
        {ipcResult && <span className="ipc-result">{ipcResult}</span>}
      </GlassPanel>
    </div>
  );
}
