import { useState, useEffect, useCallback } from "react";
import type { Component, ComponentTemplate, Project } from "../types/manifest";
import {
  setBoard,
  setSerialPort,
  setBaudRate,
  addComponent,
  updateComponent,
  removeComponent,
  listSerialPorts,
} from "../commands/projects";
import ComponentPicker from "../components/ComponentPicker";

const BOARDS = [
  "arduino:avr:uno",
  "arduino:avr:mega",
  "arduino:avr:nano",
  "arduino:sam:arduino_due_x",
  "esp32:esp32:esp32",
];

const BAUD_RATES = [9600, 19200, 38400, 57600, 115200];

interface ManifestViewProps {
  project: Project | null;
  onProjectUpdated: () => void;
}

export default function ManifestView({ project, onProjectUpdated }: ManifestViewProps) {
  const [ports, setPorts] = useState<string[]>([]);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [editingComponent, setEditingComponent] = useState<Component | null>(null);

  const refreshPorts = useCallback(() => {
    listSerialPorts().then(setPorts).catch(() => setPorts([]));
  }, []);

  useEffect(() => {
    refreshPorts();
  }, [refreshPorts]);

  if (!project) {
    return (
      <div className="view-placeholder">
        <div className="view-placeholder-title">Manifest</div>
        <div className="view-placeholder-subtitle">
          No project loaded. Create or open a project from the sidebar.
        </div>
      </div>
    );
  }

  const { manifest } = project;

  const handleBoardChange = async (board: string) => {
    await setBoard(board);
    onProjectUpdated();
  };

  const handlePortChange = async (port: string) => {
    await setSerialPort(port);
    onProjectUpdated();
  };

  const handleBaudChange = async (baud: number) => {
    await setBaudRate(baud);
    onProjectUpdated();
  };

  const handleAddComponent = async (template: ComponentTemplate) => {
    setPickerOpen(false);

    // Generate unique ID
    const baseId = template.component_type;
    let suffix = 1;
    const existingIds = new Set(manifest.components.map((c) => c.id));
    let id = `${baseId}_${suffix}`;
    while (existingIds.has(id)) {
      suffix++;
      id = `${baseId}_${suffix}`;
    }

    const pins: Record<string, number> = {};
    for (const pin of template.pins) {
      pins[pin.name] = 0;
    }

    const component: Component = {
      id,
      component_type: template.component_type,
      pins,
      label: `${template.label} ${suffix}`,
      subtype: template.optional_fields.includes("subtype") ? "esp32-cam" : undefined,
      connection: template.optional_fields.includes("connection") ? "wifi" : undefined,
      resolution: template.optional_fields.includes("resolution") ? "320x240" : undefined,
      format: template.optional_fields.includes("format") ? "jpeg" : undefined,
    };

    try {
      await addComponent(component);
      onProjectUpdated();
      setExpandedId(id);
    } catch (err) {
      console.error("Failed to add component:", err);
    }
  };

  const handleUpdateComponent = async (original_id: string, comp: Component) => {
    try {
      await updateComponent(original_id, comp);
      onProjectUpdated();
    } catch (err) {
      console.error("Failed to update component:", err);
    }
  };

  const handleRemoveComponent = async (id: string) => {
    try {
      await removeComponent(id);
      onProjectUpdated();
      if (expandedId === id) setExpandedId(null);
    } catch (err) {
      console.error("Failed to remove component:", err);
    }
  };

  const startEditing = (comp: Component) => {
    setEditingComponent({ ...comp, pins: { ...comp.pins } });
    setExpandedId(comp.id);
  };

  const cancelEditing = () => {
    setEditingComponent(null);
  };

  const saveEditing = async () => {
    if (!editingComponent) return;
    await handleUpdateComponent(expandedId!, editingComponent);
    setEditingComponent(null);
  };

  return (
    <div className="manifest-view">
      {/* Board Configuration */}
      <div className="manifest-section glass-standard">
        <div className="label" style={{ marginBottom: 12 }}>
          Board Configuration
        </div>

        <div className="manifest-field">
          <label className="manifest-field-label">Board</label>
          <select
            className="manifest-select"
            value={manifest.board}
            onChange={(e) => handleBoardChange(e.target.value)}
          >
            {BOARDS.map((b) => (
              <option key={b} value={b}>
                {b}
              </option>
            ))}
          </select>
        </div>

        <div className="manifest-field">
          <label className="manifest-field-label">Serial Port</label>
          <div className="manifest-port-row">
            <select
              className="manifest-select"
              value={manifest.serial_port}
              onChange={(e) => handlePortChange(e.target.value)}
            >
              <option value="">Select port…</option>
              {ports.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
            <button className="manifest-refresh-btn" onClick={refreshPorts} title="Refresh ports">
              ↻
            </button>
          </div>
        </div>

        <div className="manifest-field">
          <label className="manifest-field-label">Baud Rate</label>
          <select
            className="manifest-select"
            value={manifest.baud_rate}
            onChange={(e) => handleBaudChange(Number(e.target.value))}
          >
            {BAUD_RATES.map((b) => (
              <option key={b} value={b}>
                {b}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Components */}
      <div className="manifest-section">
        <div className="manifest-components-header">
          <span className="label">Components ({manifest.components.length})</span>
          <button className="manifest-add-btn" onClick={() => setPickerOpen(true)}>
            + Add
          </button>
        </div>

        {manifest.components.length === 0 ? (
          <div className="manifest-empty glass-standard">
            No components defined. Click "+ Add" to add your first component.
          </div>
        ) : (
          <div className="manifest-component-list">
            {manifest.components.map((comp) => {
              const isExpanded = expandedId === comp.id;
              const isEditing = isExpanded && editingComponent && editingComponent.id === comp.id;
              const displayComp = isEditing ? editingComponent : comp;

              return (
                <div
                  key={comp.id}
                  className={`manifest-component glass-standard ${isExpanded ? "expanded" : ""}`}
                >
                  <div
                    className="manifest-component-header"
                    onClick={() => {
                      if (isExpanded) {
                        setExpandedId(null);
                        cancelEditing();
                      } else {
                        startEditing(comp);
                      }
                    }}
                  >
                    <span className="manifest-drag-handle">≡</span>
                    <div className="manifest-component-info">
                      <span className="manifest-component-label">{comp.label}</span>
                      <span className="manifest-component-type text-secondary">
                        {comp.component_type}
                      </span>
                    </div>
                    <div className="manifest-component-pins-summary mono text-secondary">
                      {Object.entries(comp.pins)
                        .map(([k, v]) => `${k}:${v}`)
                        .join("  ")}
                    </div>
                    <span className="manifest-expand-icon">{isExpanded ? "▾" : "▸"}</span>
                  </div>

                  {isExpanded && displayComp && (
                    <div className="manifest-component-editor">
                      <div className="manifest-field">
                        <label className="manifest-field-label">Label</label>
                        <input
                          className="manifest-input"
                          value={displayComp.label}
                          onChange={(e) =>
                            setEditingComponent({ ...displayComp, label: e.target.value })
                          }
                        />
                      </div>

                      <div className="manifest-field">
                        <label className="manifest-field-label">ID</label>
                        <input
                          className="manifest-input mono"
                          value={displayComp.id}
                          onChange={(e) =>
                            setEditingComponent({ ...displayComp, id: e.target.value })
                          }
                        />
                      </div>

                      <div className="manifest-pin-editor">
                        <div className="label" style={{ marginBottom: 6, fontSize: 10 }}>
                          Pin Assignments
                        </div>
                        {Object.entries(displayComp.pins).map(([pinName, pinVal]) => (
                          <div key={pinName} className="manifest-pin-row">
                            <span className="manifest-pin-name mono">{pinName}</span>
                            <input
                              className="manifest-pin-input mono"
                              type="number"
                              min={0}
                              max={99}
                              value={pinVal}
                              onChange={(e) => {
                                const newPins = { ...displayComp.pins };
                                newPins[pinName] = Number(e.target.value) || 0;
                                setEditingComponent({ ...displayComp, pins: newPins });
                              }}
                            />
                          </div>
                        ))}
                      </div>

                      {/* Optional fields for camera, etc. */}
                      {displayComp.subtype !== undefined && (
                        <div className="manifest-field">
                          <label className="manifest-field-label">Subtype</label>
                          <input
                            className="manifest-input"
                            value={displayComp.subtype ?? ""}
                            onChange={(e) =>
                              setEditingComponent({ ...displayComp, subtype: e.target.value })
                            }
                          />
                        </div>
                      )}
                      {displayComp.connection !== undefined && (
                        <div className="manifest-field">
                          <label className="manifest-field-label">Connection</label>
                          <input
                            className="manifest-input"
                            value={displayComp.connection ?? ""}
                            onChange={(e) =>
                              setEditingComponent({ ...displayComp, connection: e.target.value })
                            }
                          />
                        </div>
                      )}
                      {displayComp.resolution !== undefined && (
                        <div className="manifest-field">
                          <label className="manifest-field-label">Resolution</label>
                          <input
                            className="manifest-input"
                            value={displayComp.resolution ?? ""}
                            onChange={(e) =>
                              setEditingComponent({ ...displayComp, resolution: e.target.value })
                            }
                          />
                        </div>
                      )}

                      <div className="manifest-editor-actions">
                        <button className="manifest-save-btn" onClick={saveEditing}>
                          Save
                        </button>
                        <button className="manifest-cancel-btn" onClick={cancelEditing}>
                          Cancel
                        </button>
                        <button
                          className="manifest-remove-btn"
                          onClick={() => handleRemoveComponent(comp.id)}
                        >
                          Remove
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <ComponentPicker
        open={pickerOpen}
        onClose={() => setPickerOpen(false)}
        onSelect={handleAddComponent}
      />
    </div>
  );
}
