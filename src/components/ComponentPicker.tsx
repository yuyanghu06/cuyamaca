import { useState, useEffect } from "react";
import type { ComponentTemplate } from "../types/manifest";
import { getComponentLibrary } from "../commands/projects";

interface ComponentPickerProps {
  open: boolean;
  onClose: () => void;
  onSelect: (template: ComponentTemplate) => void;
}

const CATEGORY_LABELS: Record<string, string> = {
  actuator: "Actuators",
  distance: "Distance / Proximity",
  motion: "Motion / Orientation",
  touch: "Touch / Tactile",
  environmental: "Environmental",
  vision: "Vision",
};

const CATEGORY_ORDER = ["actuator", "distance", "motion", "touch", "environmental", "vision"];

export default function ComponentPicker({ open, onClose, onSelect }: ComponentPickerProps) {
  const [library, setLibrary] = useState<ComponentTemplate[]>([]);

  useEffect(() => {
    if (open) {
      getComponentLibrary().then(setLibrary).catch(console.error);
    }
  }, [open]);

  if (!open) return null;

  const grouped = new Map<string, ComponentTemplate[]>();
  for (const tmpl of library) {
    const list = grouped.get(tmpl.category) ?? [];
    list.push(tmpl);
    grouped.set(tmpl.category, list);
  }

  return (
    <div className="picker-overlay" onClick={onClose}>
      <div className="picker-modal glass-strong" onClick={(e) => e.stopPropagation()}>
        <div className="picker-header">
          <span className="picker-title">Add Component</span>
          <button className="picker-close" onClick={onClose}>✕</button>
        </div>
        <div className="picker-body">
          {CATEGORY_ORDER.map((cat) => {
            const items = grouped.get(cat);
            if (!items) return null;
            return (
              <div key={cat} className="picker-category">
                <div className="label" style={{ marginBottom: 8 }}>
                  {CATEGORY_LABELS[cat] ?? cat}
                </div>
                <div className="picker-grid">
                  {items.map((tmpl) => (
                    <button
                      key={tmpl.component_type}
                      className="picker-tile glass-standard"
                      onClick={() => onSelect(tmpl)}
                    >
                      <span className="picker-tile-name">{tmpl.label}</span>
                      {tmpl.serial_output && (
                        <span className="picker-tile-output mono">
                          {tmpl.serial_output}
                        </span>
                      )}
                    </button>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
