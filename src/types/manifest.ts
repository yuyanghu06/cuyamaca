export interface Manifest {
  project: string;
  board: string;
  serial_port: string;
  baud_rate: number;
  components: Component[];
}

export interface Component {
  id: string;
  component_type: string;
  pins: Record<string, number>;
  label: string;
  subtype?: string;
  connection?: string;
  resolution?: string;
  format?: string;
}

export interface Project {
  name: string;
  path: string;
  manifest: Manifest;
  sketch: string | null;
  has_tools: boolean;
}

export interface ProjectSummary {
  name: string;
  board: string;
  component_count: number;
}

export interface PinTemplate {
  name: string;
  description: string;
}

export interface ComponentTemplate {
  component_type: string;
  category: string;
  label: string;
  pins: PinTemplate[];
  optional_fields: string[];
  serial_output: string | null;
}

export interface DiffLine {
  line_number: number;
  content: string;
  status: "added" | "removed" | "unchanged";
}

export interface GeneratedSketchResponse {
  code: string;
  diff: DiffLine[] | null;
}

export interface SerialToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, ToolParameter>;
  serial_command: string;
}

export interface ToolParameter {
  type: string;
  range?: string;
  default?: unknown;
  required: boolean;
}

export interface ChatResponse {
  text: string;
  sketch: GeneratedSketchResponse | null;
}

export interface DetectedBoard {
  port: string;
  fqbn: string | null;
  board_name: string | null;
  protocol: string;
}

export type FlashEvent =
  | { event: "compiling" }
  | { event: "uploading" }
  | { event: "succeeded"; data: { binary_size: number; max_size: number } }
  | { event: "failed"; data: { error: string } };

// ── Serial / Sensor types ──

export type SerialEvent =
  | { event: "rawLine"; data: string }
  | { event: "sensorUpdate"; data: { sensor_id: string; values: number[]; formatted: string } }
  | { event: "disconnected"; data: { error: string } };

export interface SensorSnapshot {
  sensor_id: string;
  label: string;
  component_type: string;
  values: number[];
  formatted: string;
  timestamp_ms: number;
}

export interface SensorStateSnapshot {
  sensors: SensorSnapshot[];
  formatted_text: string;
}
