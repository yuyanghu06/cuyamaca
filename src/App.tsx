import { useState } from "react";
import Sidebar from "./components/Sidebar";
import PartsPanel from "./components/PartsPanel";
import ManifestView from "./views/ManifestView";
import CodeView from "./views/CodeView";
import ChatView from "./views/ChatView";
import "./styles/globals.css";

type NavView = "manifest" | "code" | "chat";

function App() {
  const [activeView, setActiveView] = useState<NavView>("manifest");
  const [partsPanelCollapsed, setPartsPanelCollapsed] = useState(false);

  const renderView = () => {
    switch (activeView) {
      case "manifest":
        return <ManifestView />;
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
        <Sidebar activeView={activeView} onNavigate={setActiveView} />
        <main className="main-area">
          <div className="main-content">{renderView()}</div>
        </main>
        <PartsPanel
          collapsed={partsPanelCollapsed}
          onToggle={() => setPartsPanelCollapsed((c) => !c)}
        />
      </div>
    </>
  );
}

export default App;
