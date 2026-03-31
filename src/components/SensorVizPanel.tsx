import { useState, useEffect, useCallback } from "react";
import { getSensorViz } from "../commands/serial";

interface SensorVizPanelProps {
  connected: boolean;
}

export default function SensorVizPanel({ connected }: SensorVizPanelProps) {
  const [imageDataUrl, setImageDataUrl] = useState<string | null>(null);

  const fetchViz = useCallback(async () => {
    if (!connected) return;
    try {
      const data = await getSensorViz();
      if (data && data.length > 0) {
        const bytes = new Uint8Array(data);
        const blob = new Blob([bytes], { type: "image/png" });
        const url = URL.createObjectURL(blob);
        setImageDataUrl((prev) => {
          if (prev) URL.revokeObjectURL(prev);
          return url;
        });
      } else {
        setImageDataUrl(null);
      }
    } catch {
      // No connection or no spatial sensors — that's fine
    }
  }, [connected]);

  // Refresh visualization every 500ms (throttled, not every sensor update)
  useEffect(() => {
    if (!connected) {
      setImageDataUrl(null);
      return;
    }
    fetchViz();
    const interval = setInterval(fetchViz, 500);
    return () => clearInterval(interval);
  }, [connected, fetchViz]);

  // Clean up blob URL on unmount
  useEffect(() => {
    return () => {
      if (imageDataUrl) URL.revokeObjectURL(imageDataUrl);
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  if (!imageDataUrl) return null;

  return (
    <div className="sensor-viz-panel">
      <div className="sensor-viz-header">
        <span className="label">Visualizations</span>
      </div>
      <div className="sensor-viz-content">
        <img
          src={imageDataUrl}
          alt="Sensor visualization"
          className="sensor-viz-image"
        />
      </div>
    </div>
  );
}
