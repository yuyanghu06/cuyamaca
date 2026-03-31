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
