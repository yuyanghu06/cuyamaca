interface PartsPanelProps {
  collapsed: boolean;
  onToggle: () => void;
}

export default function PartsPanel({ collapsed, onToggle }: PartsPanelProps) {
  return (
    <>
      <button
        className="panel-toggle"
        onClick={onToggle}
        title={collapsed ? "Show parts panel" : "Hide parts panel"}
      >
        {collapsed ? "◧" : "▣"}
      </button>

      <aside
        className={`parts-panel glass-subtle ${collapsed ? "collapsed" : ""} ${
          !collapsed ? "force-show" : ""
        }`}
      >
        <div className="parts-header">
          <span className="label">Components</span>
        </div>
        <div className="parts-placeholder">
          No project loaded.
          <br />
          <span style={{ fontSize: 11, marginTop: 4, display: "inline-block" }}>
            Create or open a project to see components here.
          </span>
        </div>
      </aside>
    </>
  );
}
