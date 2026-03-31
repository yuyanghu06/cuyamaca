import { useRef, useEffect } from "react";
import type { SensorSnapshot } from "../types/manifest";

interface SensorStatePanelProps {
  sensors: SensorSnapshot[];
}

export default function SensorStatePanel({ sensors }: SensorStatePanelProps) {
  return (
    <div className="sensor-state-panel">
      <div className="sensor-state-header">
        <span className="label">Sensor State</span>
      </div>
      <div className="sensor-state-list">
        {sensors.length === 0 ? (
          <div className="sensor-state-empty text-secondary">
            No sensor data yet
          </div>
        ) : (
          sensors.map((s) => (
            <SensorRow key={s.sensor_id} sensor={s} />
          ))
        )}
      </div>
    </div>
  );
}

function SensorRow({ sensor }: { sensor: SensorSnapshot }) {
  const valueRef = useRef<HTMLSpanElement>(null);
  const prevValue = useRef(sensor.formatted);

  // Flash cyan on value change
  useEffect(() => {
    if (sensor.formatted !== prevValue.current && valueRef.current) {
      valueRef.current.classList.add("sensor-value-flash");
      const timeout = setTimeout(() => {
        valueRef.current?.classList.remove("sensor-value-flash");
      }, 200);
      prevValue.current = sensor.formatted;
      return () => clearTimeout(timeout);
    }
  }, [sensor.formatted]);

  const isTriggered =
    sensor.component_type === "bump_switch" && sensor.formatted === "triggered";

  return (
    <div className="sensor-row">
      <span className="sensor-label text-secondary">{sensor.label}</span>
      <span
        ref={valueRef}
        className={`sensor-value ${isTriggered ? "sensor-value-triggered" : ""}`}
      >
        {sensor.formatted}
      </span>
    </div>
  );
}
