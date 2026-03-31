import { useState, useCallback } from "react";
import GlassPanel from "./GlassPanel";
import type { DepState, InstallEvent } from "../commands/setup";
import { installDependency } from "../commands/setup";

interface Props {
  name: string;
  label: string;
  state: DepState;
  onStateChange: (state: DepState) => void;
}

export default function DependencyRow({
  name,
  label,
  state,
  onStateChange,
}: Props) {
  const [error, setError] = useState<string | null>(null);

  const handleInstall = useCallback(async () => {
    setError(null);
    onStateChange({ state: "installing", data: { progress: 0, message: "Starting..." } });

    try {
      await installDependency(name, (event: InstallEvent) => {
        switch (event.event) {
          case "progress":
            onStateChange({
              state: "installing",
              data: { progress: event.data.percent, message: event.data.message },
            });
            break;
          case "extracting":
            onStateChange({
              state: "installing",
              data: { progress: 85, message: event.data.message },
            });
            break;
          case "validating":
            onStateChange({
              state: "installing",
              data: { progress: 95, message: "Validating installation..." },
            });
            break;
          case "complete":
            onStateChange({
              state: "ready",
              data: { version: event.data.version, path: event.data.path },
            });
            break;
          case "failed":
            setError(event.data.error);
            onStateChange({ state: "missing" });
            break;
        }
      });
    } catch (err) {
      const msg = String(err);
      setError(msg);
      onStateChange({ state: "missing" });
    }
  }, [name, onStateChange]);

  const renderStatus = () => {
    switch (state.state) {
      case "ready":
        return (
          <div className="dep-row-status dep-ready">
            <span className="dep-icon">✓</span>
            <div className="dep-info">
              <span className="dep-label">{label}</span>
              <span className="dep-version">v{state.data.version}</span>
              <span className="dep-path">{state.data.path}</span>
            </div>
            <span className="dep-badge dep-badge-ready">Ready</span>
          </div>
        );

      case "missing":
        return (
          <div className="dep-row-status dep-missing">
            <span className="dep-icon dep-icon-missing">✕</span>
            <div className="dep-info">
              <span className="dep-label">{label}</span>
              <span className="dep-detail">Not installed</span>
            </div>
            <button className="dep-install-btn" onClick={handleInstall}>
              Install
            </button>
          </div>
        );

      case "unhealthy":
        return (
          <div className="dep-row-status dep-unhealthy">
            <span className="dep-icon dep-icon-missing">!</span>
            <div className="dep-info">
              <span className="dep-label">{label}</span>
              <span className="dep-detail">{state.data.reason}</span>
            </div>
            <button className="dep-install-btn" onClick={handleInstall}>
              Reinstall
            </button>
          </div>
        );

      case "installing":
        return (
          <div className="dep-row-status dep-installing">
            <span className="dep-icon dep-icon-spin">↻</span>
            <div className="dep-info">
              <span className="dep-label">{label}</span>
              <span className="dep-detail">{state.data.message}</span>
              <div className="dep-progress-bar">
                <div
                  className="dep-progress-fill"
                  style={{ width: `${Math.min(state.data.progress, 100)}%` }}
                />
              </div>
            </div>
            <span className="dep-badge dep-badge-installing">
              {Math.round(state.data.progress)}%
            </span>
          </div>
        );
    }
  };

  return (
    <GlassPanel tier="standard" className="dep-row">
      {renderStatus()}
      {error && (
        <div className="dep-error">
          {error}
          <button className="dep-retry-btn" onClick={handleInstall}>
            Retry
          </button>
        </div>
      )}
    </GlassPanel>
  );
}
