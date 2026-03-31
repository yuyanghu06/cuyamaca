import { useState, useEffect, useCallback } from "react";
import GlassPanel from "../components/GlassPanel";
import StatusDot from "../components/StatusDot";
import {
  listProviders,
  listOllamaModels,
  configureModelSlot,
  getSlotConfig,
  checkModelHealth,
  checkOllamaHealth,
  hasApiKey,
  pullOllamaModel,
  deleteOllamaModel,
} from "../commands/models";
import { detectArduinoCli } from "../commands/flash";
import type {
  ProviderInfo,
  ModelInfo,
  PullProgress,
} from "../commands/models";

// Static model lists for external providers
const EXTERNAL_MODELS: Record<string, { id: string; name: string; multimodal: boolean }[]> = {
  openai: [
    { id: "gpt-4o", name: "GPT-4o", multimodal: true },
    { id: "o3", name: "o3", multimodal: false },
  ],
  anthropic: [
    { id: "claude-sonnet-4-20250514", name: "Claude Sonnet", multimodal: true },
    { id: "claude-opus-4-20250514", name: "Claude Opus", multimodal: true },
  ],
  google: [
    { id: "gemini-1.5-pro", name: "Gemini 1.5 Pro", multimodal: true },
    { id: "gemini-2.0-flash", name: "Gemini 2.0 Flash", multimodal: true },
  ],
  mistral: [
    { id: "codestral-latest", name: "Codestral", multimodal: false },
  ],
};

type HealthStatus = "green" | "amber" | "red";

export default function SettingsView() {
  return (
    <div className="settings-view">
      <div className="settings-header">
        <h2 className="settings-title">Settings</h2>
      </div>
      <div className="settings-scroll">
        <ModelsSection />
        <OllamaModelsSection />
        <ConnectionsSection />
        <AboutSection />
      </div>
    </div>
  );
}

/* ─── Models Section ─── */

function ModelsSection() {
  const [providers, setProviders] = useState<ProviderInfo[]>([]);

  useEffect(() => {
    listProviders().then(setProviders).catch(() => {});
  }, []);

  return (
    <section className="settings-section">
      <div className="label settings-section-label">Models</div>
      <ModelSlot
        slot="code"
        label="Code Model"
        providers={providers}
        multimodalOnly={false}
      />
      <ModelSlot
        slot="runtime"
        label="Runtime Model"
        providers={providers}
        multimodalOnly={true}
      />
    </section>
  );
}

interface ModelSlotProps {
  slot: "code" | "runtime";
  label: string;
  providers: ProviderInfo[];
  multimodalOnly: boolean;
}

function ModelSlot({ slot, label, providers, multimodalOnly }: ModelSlotProps) {
  const [provider, setProvider] = useState("ollama");
  const [model, setModel] = useState("");
  const [ollamaModels, setOllamaModels] = useState<ModelInfo[]>([]);
  const [apiKey, setApiKey] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [health, setHealth] = useState<HealthStatus>("red");
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [multimodalWarning, setMultimodalWarning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load current config
  useEffect(() => {
    getSlotConfig(slot).then((config) => {
      if (config) {
        setProvider(config.provider);
        setModel(config.model);
        setMultimodalWarning(config.multimodal_warning);
      }
    }).catch(() => {});

    checkModelHealth(slot).then((ok) => setHealth(ok ? "green" : "red")).catch(() => {});
  }, [slot]);

  // Load Ollama models when provider is ollama
  useEffect(() => {
    if (provider === "ollama") {
      listOllamaModels().then(setOllamaModels).catch(() => setOllamaModels([]));
    }
  }, [provider]);

  // Check if we have an API key for this provider
  useEffect(() => {
    if (provider !== "ollama") {
      hasApiKey(provider).then(setHasKey).catch(() => setHasKey(false));
    }
  }, [provider]);

  const availableModels = provider === "ollama"
    ? (multimodalOnly ? ollamaModels.filter((m) => m.multimodal) : ollamaModels)
    : (EXTERNAL_MODELS[provider] ?? []).filter((m) => !multimodalOnly || m.multimodal);

  const handleSave = useCallback(async () => {
    if (!model) return;
    setSaving(true);
    setError(null);
    try {
      const result = await configureModelSlot(
        slot,
        provider,
        model,
        apiKey || undefined,
      );
      setMultimodalWarning(result.multimodal_warning);
      // Test health after save
      const ok = await checkModelHealth(slot);
      setHealth(ok ? "green" : "red");
      if (apiKey) {
        setHasKey(true);
        setApiKey("");
      }
    } catch (err) {
      setError(String(err));
      setHealth("red");
    } finally {
      setSaving(false);
    }
  }, [slot, provider, model, apiKey]);

  const handleTest = useCallback(async () => {
    setTesting(true);
    try {
      const ok = await checkModelHealth(slot);
      setHealth(ok ? "green" : "red");
    } catch {
      setHealth("red");
    } finally {
      setTesting(false);
    }
  }, [slot]);

  const requiresKey = providers.find((p) => p.id === provider)?.requires_key ?? false;

  return (
    <GlassPanel tier="standard" className="settings-card">
      <div className="settings-card-header">
        <span className="settings-card-title">{label}</span>
        <span className="settings-card-status">
          <StatusDot status={health} />
          <span className="text-secondary" style={{ fontSize: 11 }}>
            {health === "green" ? "Connected" : health === "amber" ? "Checking…" : "Unreachable"}
          </span>
        </span>
      </div>

      <div className="settings-card-row">
        <div className="settings-field">
          <label className="label">Provider</label>
          <select
            className="settings-select"
            value={provider}
            onChange={(e) => {
              setProvider(e.target.value);
              setModel("");
              setError(null);
            }}
          >
            {providers.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </div>
        <div className="settings-field">
          <label className="label">Model</label>
          <select
            className="settings-select"
            value={model}
            onChange={(e) => setModel(e.target.value)}
          >
            <option value="">Select model…</option>
            {availableModels.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name ?? m.id}
                {multimodalOnly && m.multimodal ? " 📷" : ""}
              </option>
            ))}
          </select>
        </div>
      </div>

      {requiresKey && (
        <div className="settings-card-row">
          <div className="settings-field" style={{ flex: 1 }}>
            <label className="label">
              API Key {hasKey && <span className="text-secondary">(stored)</span>}
            </label>
            <input
              type="password"
              className="settings-input"
              placeholder={hasKey ? "••••••••••" : "Enter API key"}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </div>
        </div>
      )}

      {multimodalOnly && multimodalWarning && (
        <div className="settings-warning">
          ⚠ This model does not support image input. Camera frames and sensor
          visualizations will be excluded from context.
        </div>
      )}

      {error && <div className="settings-error">{error}</div>}

      <div className="settings-card-actions">
        <button
          className="settings-btn settings-btn-primary"
          onClick={handleSave}
          disabled={!model || saving}
        >
          {saving ? "Saving…" : "Save"}
        </button>
        <button
          className="settings-btn"
          onClick={handleTest}
          disabled={testing}
        >
          {testing ? "Testing…" : "Test"}
        </button>
      </div>
    </GlassPanel>
  );
}

/* ─── Ollama Models Section ─── */

function OllamaModelsSection() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [pullName, setPullName] = useState("");
  const [pulling, setPulling] = useState(false);
  const [pullPercent, setPullPercent] = useState(0);
  const [pullStatus, setPullStatus] = useState("");
  const [pullError, setPullError] = useState<string | null>(null);

  const refreshModels = useCallback(async () => {
    try {
      const list = await listOllamaModels();
      setModels(list);
    } catch {
      setModels([]);
    }
  }, []);

  useEffect(() => {
    refreshModels();
  }, [refreshModels]);

  const handlePull = useCallback(async () => {
    const name = pullName.trim();
    if (!name || pulling) return;
    setPulling(true);
    setPullError(null);
    setPullPercent(0);
    setPullStatus("Starting…");

    try {
      await pullOllamaModel(name, (event: PullProgress) => {
        switch (event.event) {
          case "started":
            setPullStatus("Starting download…");
            break;
          case "downloading": {
            const pct = event.data.total > 0
              ? Math.round((event.data.completed / event.data.total) * 100)
              : 0;
            setPullPercent(pct);
            setPullStatus(`Downloading… ${pct}%`);
            break;
          }
          case "verifying":
            setPullPercent(100);
            setPullStatus("Verifying…");
            break;
          case "succeeded":
            setPullStatus("Complete!");
            setPulling(false);
            setPullName("");
            refreshModels();
            break;
          case "failed":
            setPullError(event.data.error);
            setPulling(false);
            break;
        }
      });
    } catch (err) {
      setPullError(String(err));
    } finally {
      setPulling(false);
    }
  }, [pullName, pulling, refreshModels]);

  const handleDelete = useCallback(async (name: string) => {
    if (!confirm(`Delete model "${name}"?`)) return;
    try {
      await deleteOllamaModel(name);
      refreshModels();
    } catch (err) {
      console.error("Delete failed:", err);
    }
  }, [refreshModels]);

  return (
    <section className="settings-section">
      <div className="label settings-section-label">Ollama Models</div>
      <GlassPanel tier="standard" className="settings-card">
        {models.length === 0 ? (
          <div className="settings-empty">No Ollama models installed.</div>
        ) : (
          <div className="ollama-model-list">
            {models.map((m) => (
              <div key={m.id} className="ollama-model-row">
                <span className="ollama-model-name">{m.name}</span>
                {m.multimodal && <span className="ollama-model-badge">multimodal</span>}
                <button
                  className="ollama-model-delete"
                  onClick={() => handleDelete(m.id)}
                  title="Delete model"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        )}

        <div className="ollama-pull-row">
          <input
            className="settings-input"
            placeholder="Model name (e.g. llava:13b)"
            value={pullName}
            onChange={(e) => setPullName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handlePull()}
            disabled={pulling}
          />
          <button
            className="settings-btn settings-btn-primary"
            onClick={handlePull}
            disabled={pulling || !pullName.trim()}
          >
            {pulling ? "Pulling…" : "Pull"}
          </button>
        </div>

        {pulling && (
          <div className="ollama-pull-progress">
            <div className="ollama-pull-bar">
              <div
                className="ollama-pull-bar-fill"
                style={{ width: `${pullPercent}%` }}
              />
            </div>
            <span className="ollama-pull-status">{pullStatus}</span>
          </div>
        )}

        {pullError && <div className="settings-error">{pullError}</div>}
      </GlassPanel>
    </section>
  );
}

/* ─── Connections Section ─── */

function ConnectionsSection() {
  const [ollamaOk, setOllamaOk] = useState(false);
  const [arduinoOk, setArduinoOk] = useState(false);

  useEffect(() => {
    checkOllamaHealth().then(setOllamaOk).catch(() => setOllamaOk(false));
    detectArduinoCli().then(setArduinoOk).catch(() => setArduinoOk(false));
  }, []);

  return (
    <section className="settings-section">
      <div className="label settings-section-label">Connections</div>
      <GlassPanel tier="standard" className="settings-card">
        <div className="settings-connection-row">
          <StatusDot status={ollamaOk ? "green" : "red"} />
          <div className="settings-connection-info">
            <span className="settings-connection-label">Ollama</span>
            <span className="text-secondary" style={{ fontSize: 11 }}>
              {ollamaOk ? "Running on localhost:11434" : "Not running"}
            </span>
          </div>
        </div>
        <div className="settings-connection-row">
          <StatusDot status={arduinoOk ? "green" : "red"} />
          <div className="settings-connection-info">
            <span className="settings-connection-label">arduino-cli</span>
            <span className="text-secondary" style={{ fontSize: 11 }}>
              {arduinoOk ? "Installed" : "Not found"}
            </span>
          </div>
        </div>
      </GlassPanel>
    </section>
  );
}

/* ─── About Section ─── */

function AboutSection() {
  const [reduceTransparency, setReduceTransparency] = useState(
    () => document.documentElement.getAttribute("data-reduce-transparency") === "true",
  );
  const [reduceMotion, setReduceMotion] = useState(
    () => document.documentElement.getAttribute("data-reduce-motion") === "true",
  );

  const toggleTransparency = () => {
    const next = !reduceTransparency;
    setReduceTransparency(next);
    document.documentElement.setAttribute("data-reduce-transparency", String(next));
    localStorage.setItem("cuyamaca-reduce-transparency", String(next));
  };

  const toggleMotion = () => {
    const next = !reduceMotion;
    setReduceMotion(next);
    document.documentElement.setAttribute("data-reduce-motion", String(next));
    localStorage.setItem("cuyamaca-reduce-motion", String(next));
  };

  return (
    <section className="settings-section">
      <div className="label settings-section-label">About</div>
      <GlassPanel tier="standard" className="settings-card">
        <div className="settings-about-header">
          <span className="settings-about-name">Cuyamaca</span>
          <span className="text-secondary">v0.1.0</span>
        </div>
        <p className="text-secondary" style={{ margin: "4px 0 16px", fontSize: 13 }}>
          Natural language Arduino control
        </p>

        <div className="label" style={{ marginBottom: 8 }}>Accessibility</div>
        <label className="settings-toggle-row">
          <input
            type="checkbox"
            role="switch"
            aria-checked={reduceTransparency}
            checked={reduceTransparency}
            onChange={toggleTransparency}
          />
          <span>Reduce transparency</span>
        </label>
        <label className="settings-toggle-row">
          <input
            type="checkbox"
            role="switch"
            aria-checked={reduceMotion}
            checked={reduceMotion}
          onChange={toggleMotion}
          />
          <span>Reduce motion</span>
        </label>
      </GlassPanel>
    </section>
  );
}
