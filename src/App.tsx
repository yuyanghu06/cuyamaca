import { useState, useCallback } from "react";
import Sidebar from "./components/Sidebar";
import PartsPanel from "./components/PartsPanel";
import ManifestView from "./views/ManifestView";
import CodeView from "./views/CodeView";
import ChatView from "./views/ChatView";
import { openProject } from "./commands/projects";
import type { Project } from "./types/manifest";
import "./styles/globals.css";

type NavView = "manifest" | "code" | "chat";

function App() {
  const [activeView, setActiveView] = useState<NavView>("manifest");
  const [partsPanelCollapsed, setPartsPanelCollapsed] = useState(false);
  const [activeProject, setActiveProject] = useState<Project | null>(null);

  const refreshActiveProject = useCallback(async () => {
    if (!activeProject) return;
    try {
      const updated = await openProject(activeProject.name);
      setActiveProject(updated);
    } catch (err) {
      console.error("Failed to refresh project:", err);
    }
  }, [activeProject]);

  const renderView = () => {
    switch (activeView) {
      case "manifest":
        return (
          <ManifestView
            project={activeProject}
            onProjectUpdated={refreshActiveProject}
          />
        );
      case "code":
        return <CodeView />;
      case "chat":
        return <ChatView />;
    }
  };

  return (
    <>
      <div className="app-background" />
      <div className="app-layout">
        <Sidebar
          activeView={activeView}
          onNavigate={setActiveView}
          activeProject={activeProject}
          onProjectOpened={setActiveProject}
        />
        <main className="main-area">
          <div className="main-content">{renderView()}</div>
        </main>
        <PartsPanel
          collapsed={partsPanelCollapsed}
          onToggle={() => setPartsPanelCollapsed((c) => !c)}
          components={activeProject?.manifest.components ?? []}
        />
      </div>
    </>
  );
}

export default App;
