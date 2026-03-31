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
import { restartOllama } from "../commands/setup";
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

// Curated catalog of well-known Ollama models.
// These are shown in the selector even when not yet installed, so users can
// pick a model and pull it in one step from the Ollama Models section.
interface OllamaCatalogEntry {
  id: string;      // model tag used by ollama pull / run
  name: string;    // human-readable label
  multimodal: boolean;
  size?: string;   // rough param count for display
}

const OLLAMA_CATALOG: OllamaCatalogEntry[] = [
  // ── Vision / Multimodal ──────────────────────────────────────────────────
  { id: "llava:7b",              name: "LLaVA 1.6 (7B)",             multimodal: true,  size: "7B"  },
  { id: "llava:13b",             name: "LLaVA 1.6 (13B)",            multimodal: true,  size: "13B" },
  { id: "llava:34b",             name: "LLaVA 1.6 (34B)",            multimodal: true,  size: "34B" },
  { id: "llava-llama3:8b",       name: "LLaVA-LLaMA3 (8B)",          multimodal: true,  size: "8B"  },
  { id: "llava-phi3:3.8b",       name: "LLaVA-Phi3 (3.8B)",          multimodal: true,  size: "3.8B"},
  { id: "bakllava:7b",           name: "BakLLaVA (7B)",               multimodal: true,  size: "7B"  },
  { id: "moondream:latest",      name: "Moondream 2",                 multimodal: true,  size: "1.8B"},
  { id: "llama3.2-vision:11b",   name: "Llama 3.2 Vision (11B)",     multimodal: true,  size: "11B" },
  { id: "llama3.2-vision:90b",   name: "Llama 3.2 Vision (90B)",     multimodal: true,  size: "90B" },
  // ── General / Code ───────────────────────────────────────────────────────
  { id: "llama3.2:1b",           name: "Llama 3.2 (1B)",             multimodal: false, size: "1B"  },
  { id: "llama3.2:3b",           name: "Llama 3.2 (3B)",             multimodal: false, size: "3B"  },
  { id: "llama3.1:8b",           name: "Llama 3.1 (8B)",             multimodal: false, size: "8B"  },
  { id: "llama3.1:70b",          name: "Llama 3.1 (70B)",            multimodal: false, size: "70B" },
  { id: "llama3:8b",             name: "Llama 3 (8B)",               multimodal: false, size: "8B"  },
  { id: "mistral:7b",            name: "Mistral (7B)",               multimodal: false, size: "7B"  },
  { id: "mistral-nemo:12b",      name: "Mistral Nemo (12B)",         multimodal: false, size: "12B" },
  { id: "codestral:22b",         name: "Codestral (22B)",            multimodal: false, size: "22B" },
  { id: "qwen2.5-coder:7b",      name: "Qwen 2.5 Coder (7B)",        multimodal: false, size: "7B"  },
  { id: "qwen2.5-coder:32b",     name: "Qwen 2.5 Coder (32B)",       multimodal: false, size: "32B" },
  { id: "deepseek-coder:6.7b",   name: "DeepSeek Coder (6.7B)",      multimodal: false, size: "6.7B"},
  { id: "deepseek-coder-v2:16b", name: "DeepSeek Coder V2 (16B)",    multimodal: false, size: "16B" },
  { id: "phi4:14b",              name: "Phi-4 (14B)",                multimodal: false, size: "14B" },
  { id: "phi3:3.8b",             name: "Phi-3 (3.8B)",               multimodal: false, size: "3.8B"},
  { id: "phi3:14b",              name: "Phi-3 (14B)",                multimodal: false, size: "14B" },
  { id: "gemma2:9b",             name: "Gemma 2 (9B)",               multimodal: false, size: "9B"  },
  { id: "gemma2:27b",            name: "Gemma 2 (27B)",              multimodal: false, size: "27B" },
  { id: "codegemma:7b",          name: "CodeGemma (7B)",             multimodal: false, size: "7B"  },
  { id: "starcoder2:7b",         name: "StarCoder2 (7B)",            multimodal: false, size: "7B"  },
];

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
  const [installedOllamaModels, setInstalledOllamaModels] = useState<ModelInfo[]>([]);
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

  // Load installed Ollama models when provider is ollama
  useEffect(() => {
    if (provider === "ollama") {
      listOllamaModels().then(setInstalledOllamaModels).catch(() => setInstalledOllamaModels([]));
    }
  }, [provider]);

  // Check if we have an API key for this provider
  useEffect(() => {
    if (provider !== "ollama") {
      hasApiKey(provider).then(setHasKey).catch(() => setHasKey(false));
    }
  }, [provider]);

  // Build the model list for Ollama: installed first, then uninstalled catalog entries
  const installedIds = new Set(installedOllamaModels.map((m) => m.id));

  const catalogFiltered = OLLAMA_CATALOG.filter(
    (m) => !multimodalOnly || m.multimodal,
  );

  // Installed models from the live Ollama list (may include user-pulled models not in catalog)
  const installedModels = installedOllamaModels
    .filter((m) => !multimodalOnly || m.multimodal)
    .map((m) => ({ id: m.id, name: m.name ?? m.id, multimodal: m.multimodal, installed: true }));

  // Catalog entries not yet installed
  const catalogNotInstalled = catalogFiltered
    .filter((m) => !installedIds.has(m.id))
    .map((m) => ({ ...m, installed: false }));

  const externalModels = (EXTERNAL_MODELS[provider] ?? []).filter(
    (m) => !multimodalOnly || m.multimodal,
  );

  // Is the currently selected Ollama model NOT yet installed?
  const isUninstalledOllamaModel =
    provider === "ollama" &&
    model !== "" &&
    !installedIds.has(model);

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
      if (!ok && provider === "ollama" && !installedIds.has(model)) {
        setError(`"${model}" is not installed. Pull it in the Ollama Models section below, then test again.`);
      }
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
  }, [slot, provider, model, apiKey, installedIds]);

  const handleTest = useCallback(async () => {
    setTesting(true);
    setError(null);
    try {
      const ok = await checkModelHealth(slot);
      setHealth(ok ? "green" : "red");
      if (!ok && provider === "ollama" && !installedIds.has(model)) {
        setError(`"${model}" is not installed. Pull it in the Ollama Models section below first.`);
      } else if (!ok) {
        setError("Model unreachable. Check that Ollama is running and the model name is correct.");
      }
    } catch {
      setHealth("red");
    } finally {
      setTesting(false);
    }
  }, [slot, provider, model, installedIds]);

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
            {provider === "ollama" ? (
              <>
                {installedModels.length > 0 && (
                  <optgroup label="Installed">
                    {installedModels.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.name}{m.multimodal ? " 📷" : ""}
                      </option>
                    ))}
                  </optgroup>
                )}
                {catalogNotInstalled.length > 0 && (
                  <optgroup label="Available to Pull">
                    {catalogNotInstalled.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.name}{m.multimodal ? " 📷" : ""}{m.size ? ` — ${m.size}` : ""}
                      </option>
                    ))}
                  </optgroup>
                )}
              </>
            ) : (
              externalModels.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))
            )}
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

      {isUninstalledOllamaModel && (
        <div className="settings-warning">
          ⬇ "{model}" is not yet installed. Save to configure the slot, then pull the model in the <strong>Ollama Models</strong> section below before using it.
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
          disabled={testing || !model}
          title={!model ? "Select a model first" : "Check if model is reachable"}
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
  const [restarting, setRestarting] = useState(false);

  const refreshHealth = useCallback(async () => {
    checkOllamaHealth().then(setOllamaOk).catch(() => setOllamaOk(false));
    detectArduinoCli().then(setArduinoOk).catch(() => setArduinoOk(false));
  }, []);

  useEffect(() => {
    refreshHealth();
  }, [refreshHealth]);

  const handleRestartOllama = useCallback(async () => {
    setRestarting(true);
    try {
      await restartOllama();
      await refreshHealth();
    } catch (err) {
      console.error("Restart failed:", err);
    } finally {
      setRestarting(false);
    }
  }, [refreshHealth]);

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
          <button
            className="settings-btn"
            onClick={handleRestartOllama}
            disabled={restarting}
            style={{ marginLeft: "auto" }}
          >
            {restarting ? "Restarting…" : "Restart"}
          </button>
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
