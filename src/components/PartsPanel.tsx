import type { Component } from "../types/manifest";

interface PartsPanelProps {
  collapsed: boolean;
  onToggle: () => void;
  components: Component[];
  onComponentClick?: (id: string) => void;
}

const CATEGORY_MAP: Record<string, string> = {
  dc_motor: "actuator",
  servo: "actuator",
  stepper_motor: "actuator",
  relay: "actuator",
  led: "actuator",
  ultrasonic: "distance",
  ir_distance: "distance",
  lidar_serial: "distance",
  imu: "motion",
  magnetometer: "motion",
  encoder: "motion",
  bump_switch: "touch",
  line_sensor_array: "touch",
  force_sensor: "touch",
  temp_humidity: "environmental",
  barometer: "environmental",
  light: "environmental",
  gas: "environmental",
  camera: "vision",
};

const CATEGORY_LABELS: Record<string, string> = {
  actuator: "Actuators",
  distance: "Sensors — Distance",
  motion: "Sensors — Motion",
  touch: "Sensors — Touch",
  environmental: "Sensors — Environmental",
  vision: "Vision",
};

const CATEGORY_ORDER = ["actuator", "distance", "motion", "touch", "environmental", "vision"];

function primaryPinDisplay(comp: Component): string {
  const entries = Object.entries(comp.pins);
  if (entries.length === 0) {
    return comp.connection ?? "";
  }
  const [key, val] = entries[0];
  return `${key}:${val}`;
}

export default function PartsPanel({
  collapsed,
  onToggle,
  components,
  onComponentClick,
}: PartsPanelProps) {
  const grouped = new Map<string, Component[]>();
  for (const comp of components) {
    const cat = CATEGORY_MAP[comp.component_type] ?? "actuator";
    const list = grouped.get(cat) ?? [];
    list.push(comp);
    grouped.set(cat, list);
  }

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

        {components.length === 0 ? (
          <div className="parts-placeholder">
            No components defined.
            <br />
            <span style={{ fontSize: 11, marginTop: 4, display: "inline-block" }}>
              Add components from the Manifest view.
            </span>
          </div>
        ) : (
          <div className="parts-grouped">
            {CATEGORY_ORDER.map((cat) => {
              const items = grouped.get(cat);
              if (!items || items.length === 0) return null;
              return (
                <div key={cat} className="parts-group">
                  <div className="parts-group-label label">{CATEGORY_LABELS[cat] ?? cat}</div>
                  {items.map((comp) => (
                    <div
                      key={comp.id}
                      className="parts-row"
                      onClick={() => onComponentClick?.(comp.id)}
                    >
                      <span className="parts-row-label">{comp.label}</span>
                      <span className="parts-row-pin mono text-secondary">
                        {primaryPinDisplay(comp)}
                      </span>
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        )}
      </aside>
    </>
  );
}
