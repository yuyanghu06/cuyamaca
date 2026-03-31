import { useState, useEffect, useCallback } from "react";
import StatusDot from "./StatusDot";
import {
  checkOllamaHealth,
  checkModelHealth,
} from "../commands/models";
import {
  listProjects,
  createProject,
  openProject,
} from "../commands/projects";
import { detectArduinoCli } from "../commands/flash";
import type { ProjectSummary, Project } from "../types/manifest";

type NavView = "manifest" | "code" | "chat";

interface SidebarProps {
  activeView: NavView;
  onNavigate: (view: NavView) => void;
  activeProject: Project | null;
  onProjectOpened: (project: Project) => void;
}

type HealthStatus = "green" | "amber" | "red";

const navItems: { id: NavView; label: string; icon: string }[] = [
  { id: "manifest", label: "Manifest", icon: "⚙" },
  { id: "code", label: "Code", icon: "⟨⟩" },
  { id: "chat", label: "Chat", icon: "◉" },
];

export default function Sidebar({
  activeView,
  onNavigate,
  activeProject,
  onProjectOpened,
}: SidebarProps) {
  const [ollamaStatus, setOllamaStatus] = useState<HealthStatus>("red");
  const [arduinoCliStatus, setArduinoCliStatus] = useState<HealthStatus>("red");
  const [codeModelStatus, setCodeModelStatus] = useState<HealthStatus>("red");
  const [runtimeModelStatus, setRuntimeModelStatus] = useState<HealthStatus>("red");
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [showNewProject, setShowNewProject] = useState(false);
  const [newName, setNewName] = useState("");
  const [newBoard, setNewBoard] = useState("arduino:avr:uno");

  const refreshProjects = useCallback(async () => {
    try {
      const list = await listProjects();
      setProjects(list);
    } catch {
      setProjects([]);
    }
  }, []);

  const pollHealth = useCallback(async () => {
    try {
      const ollamaOk = await checkOllamaHealth();
      setOllamaStatus(ollamaOk ? "green" : "red");
    } catch {
      setOllamaStatus("red");
    }

    try {
      const cliOk = await detectArduinoCli();
      setArduinoCliStatus(cliOk ? "green" : "red");
    } catch {
      setArduinoCliStatus("red");
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
    refreshProjects();
    pollHealth();
    const interval = setInterval(pollHealth, 30_000);
    const onFocus = () => { pollHealth(); refreshProjects(); };
    window.addEventListener("focus", onFocus);
    return () => {
      clearInterval(interval);
      window.removeEventListener("focus", onFocus);
    };
  }, [pollHealth, refreshProjects]);

  const handleCreateProject = async () => {
    const trimmed = newName.trim().replace(/\s+/g, "-").toLowerCase();
    if (!trimmed) return;
    try {
      await createProject(trimmed, newBoard);
      const project = await openProject(trimmed);
      onProjectOpened(project);
      setShowNewProject(false);
      setNewName("");
      refreshProjects();
    } catch (err) {
      console.error("Failed to create project:", err);
    }
  };

  const handleOpenProject = async (name: string) => {
    try {
      const project = await openProject(name);
      onProjectOpened(project);
    } catch (err) {
      console.error("Failed to open project:", err);
    }
  };

  const healthItems: { label: string; status: HealthStatus }[] = [
    { label: "Ollama", status: ollamaStatus },
    { label: "arduino-cli", status: arduinoCliStatus },
    { label: "Code Model", status: codeModelStatus },
    { label: "Runtime Model", status: runtimeModelStatus },
  ];

  return (
    <aside className="sidebar glass-subtle">
      <div className="sidebar-header">
        <div className="sidebar-app-name">Cuyamaca</div>
      </div>

      {/* Project List */}
      <div className="sidebar-projects">
        <div className="label" style={{ padding: "4px 12px 6px", fontSize: 10 }}>
          Projects
        </div>
        {projects.map((p) => (
          <div
            key={p.name}
            className={`nav-item ${activeProject?.name === p.name ? "active" : ""}`}
            onClick={() => handleOpenProject(p.name)}
          >
            <span className="nav-icon">◆</span>
            <span className="nav-label">
              <span>{p.name}</span>
              <span className="project-meta text-secondary">
                {p.board.split(":").pop()} · {p.component_count}
              </span>
            </span>
          </div>
        ))}

        {showNewProject ? (
          <div className="sidebar-new-project">
            <input
              className="sidebar-input"
              placeholder="project-name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleCreateProject()}
              autoFocus
            />
            <select
              className="sidebar-select"
              value={newBoard}
              onChange={(e) => setNewBoard(e.target.value)}
            >
              <option value="arduino:avr:uno">Uno</option>
              <option value="arduino:avr:mega">Mega</option>
              <option value="arduino:avr:nano">Nano</option>
              <option value="esp32:esp32:esp32">ESP32</option>
            </select>
            <div className="sidebar-new-actions">
              <button className="sidebar-create-btn" onClick={handleCreateProject}>
                Create
              </button>
              <button
                className="sidebar-cancel-btn"
                onClick={() => {
                  setShowNewProject(false);
                  setNewName("");
                }}
              >
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <div
            className="nav-item sidebar-new-trigger"
            onClick={() => setShowNewProject(true)}
          >
            <span className="nav-icon">+</span>
            <span className="nav-label">New Project</span>
          </div>
        )}
      </div>

      {/* Navigation */}
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
