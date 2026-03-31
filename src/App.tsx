import { useState, useCallback, useEffect } from "react";
import Sidebar from "./components/Sidebar";
import PartsPanel from "./components/PartsPanel";
import ManifestView from "./views/ManifestView";
import CodeView from "./views/CodeView";
import ChatView from "./views/ChatView";
import SettingsView from "./views/SettingsView";
import SetupWizard from "./views/SetupWizard";
import { openProject } from "./commands/projects";
import { isSetupComplete, checkDependencies } from "./commands/setup";
import type { Project, GeneratedSketchResponse } from "./types/manifest";
import "./styles/globals.css";

type NavView = "manifest" | "code" | "chat" | "settings";

function App() {
  const [activeView, setActiveView] = useState<NavView>("manifest");
  const [partsPanelCollapsed, setPartsPanelCollapsed] = useState(false);
  const [activeProject, setActiveProject] = useState<Project | null>(null);
  const [pendingSketch, setPendingSketch] = useState<GeneratedSketchResponse | null>(null);
  const [showWizard, setShowWizard] = useState<boolean | null>(null);

  // Check if first-run wizard should be shown
  useEffect(() => {
    (async () => {
      try {
        const complete = await isSetupComplete();
        if (complete) {
          setShowWizard(false);
          return;
        }
        // Not marked complete: check if deps are actually available
        const status = await checkDependencies();
        const allReady =
          status.ollama.state === "ready" &&
          status.arduinoCli.state === "ready";
        setShowWizard(!allReady);
      } catch {
        setShowWizard(false);
      }
    })();
  }, []);

  const refreshActiveProject = useCallback(async () => {
    if (!activeProject) return;
    try {
      const updated = await openProject(activeProject.name);
      setActiveProject(updated);
    } catch (err) {
      console.error("Failed to refresh project:", err);
    }
  }, [activeProject]);

  const handlePendingSketch = useCallback(
    (sketch: GeneratedSketchResponse | null) => {
      setPendingSketch(sketch);
      if (sketch) {
        setActiveView("code");
      }
    },
    [],
  );

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
        return (
          <CodeView
            project={activeProject}
            onProjectUpdated={refreshActiveProject}
            pendingSketch={pendingSketch}
            onPendingSketch={setPendingSketch}
          />
        );
      case "chat":
        return (
          <ChatView
            project={activeProject}
            onPendingSketch={handlePendingSketch}
            onSwitchToCode={() => setActiveView("code")}
          />
        );
      case "settings":
        return <SettingsView />;
    }
  };

  // Show nothing while checking setup status
  if (showWizard === null) {
    return (
      <>
        <div className="app-background" />
        <div className="setup-wizard">
          <div className="setup-wizard-inner">
            <div className="setup-header">
              <div className="setup-icon">◈</div>
              <p className="setup-subtitle">Loading...</p>
            </div>
          </div>
        </div>
      </>
    );
  }

  // Show first-run wizard if deps are missing
  if (showWizard) {
    return (
      <>
        <div className="app-background" />
        <SetupWizard onComplete={() => setShowWizard(false)} />
      </>
    );
  }

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
