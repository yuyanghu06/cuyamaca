import StatusDot from "./StatusDot";

type NavView = "manifest" | "code" | "chat";

interface SidebarProps {
  activeView: NavView;
  onNavigate: (view: NavView) => void;
}

const navItems: { id: NavView; label: string; icon: string }[] = [
  { id: "manifest", label: "Manifest", icon: "⚙" },
  { id: "code", label: "Code", icon: "⟨⟩" },
  { id: "chat", label: "Chat", icon: "◉" },
];

const healthItems = [
  { label: "Ollama", status: "red" as const },
  { label: "arduino-cli", status: "red" as const },
  { label: "Code Model", status: "red" as const },
  { label: "Runtime Model", status: "red" as const },
];

export default function Sidebar({ activeView, onNavigate }: SidebarProps) {
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
