import { useState, useEffect, useCallback } from "react";
import StatusDot from "./StatusDot";
import {
  checkOllamaHealth,
  checkModelHealth,
} from "../commands/models";

type NavView = "manifest" | "code" | "chat";

interface SidebarProps {
  activeView: NavView;
  onNavigate: (view: NavView) => void;
}

type HealthStatus = "green" | "amber" | "red";

const navItems: { id: NavView; label: string; icon: string }[] = [
  { id: "manifest", label: "Manifest", icon: "⚙" },
  { id: "code", label: "Code", icon: "⟨⟩" },
  { id: "chat", label: "Chat", icon: "◉" },
];

export default function Sidebar({ activeView, onNavigate }: SidebarProps) {
  const [ollamaStatus, setOllamaStatus] = useState<HealthStatus>("red");
  const [codeModelStatus, setCodeModelStatus] = useState<HealthStatus>("red");
  const [runtimeModelStatus, setRuntimeModelStatus] = useState<HealthStatus>("red");

  const pollHealth = useCallback(async () => {
    try {
      const ollamaOk = await checkOllamaHealth();
      setOllamaStatus(ollamaOk ? "green" : "red");
    } catch {
      setOllamaStatus("red");
    }

    try {
      const codeOk = await checkModelHealth("code");
      setCodeModelStatus(codeOk ? "green" : "red");
    } catch {
      setCodeModelStatus("red");
    }

    try {
      const runtimeOk = await checkModelHealth("runtime");
      setRuntimeModelStatus(runtimeOk ? "green" : "red");
    } catch {
      setRuntimeModelStatus("red");
    }
  }, []);

  useEffect(() => {
    pollHealth();
    const interval = setInterval(pollHealth, 30_000);

    const onFocus = () => pollHealth();
    window.addEventListener("focus", onFocus);

    return () => {
      clearInterval(interval);
      window.removeEventListener("focus", onFocus);
    };
  }, [pollHealth]);

  const healthItems: { label: string; status: HealthStatus }[] = [
    { label: "Ollama", status: ollamaStatus },
    { label: "arduino-cli", status: "red" },
    { label: "Code Model", status: codeModelStatus },
    { label: "Runtime Model", status: runtimeModelStatus },
  ];

  return (
    <aside className="sidebar glass-subtle">
      <div className="sidebar-header">
        <div className="sidebar-app-name">Cuyamaca</div>
      </div>

      <nav className="sidebar-nav">
        <div className="label" style={{ padding: "4px 12px 8px", fontSize: 10 }}>
          Views
        </div>
        {navItems.map((item) => (
          <div
            key={item.id}
            className={`nav-item ${activeView === item.id ? "active" : ""}`}
            onClick={() => onNavigate(item.id)}
          >
            <span className="nav-icon">{item.icon}</span>
            <span className="nav-label">{item.label}</span>
          </div>
        ))}
      </nav>

      <div className="sidebar-health">
        <div className="label" style={{ fontSize: 10, marginBottom: 2 }}>
          Services
        </div>
        {healthItems.map((item) => (
          <div key={item.label} className="health-row">
            <StatusDot status={item.status} />
            <span className="health-label">{item.label}</span>
          </div>
        ))}
      </div>
    </aside>
  );
}
