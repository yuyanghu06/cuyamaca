import React, { useState, useRef, useCallback } from "react";
import {
  generateSketch,
  uploadSketch,
  approveSketch,
  rejectSketch,
} from "../commands/codegen";
import { flashSketch } from "../commands/flash";
import { openRuntimeWindow } from "../commands/runtime";
import type {
  Project,
  GeneratedSketchResponse,
  DiffLine,
  FlashEvent,
} from "../types/manifest";
import GlassPanel from "../components/GlassPanel";

type FlashStatus =
  | { state: "idle" }
  | { state: "compiling" }
  | { state: "uploading" }
  | { state: "succeeded"; binarySize: number; maxSize: number }
  | { state: "failed"; error: string };

interface CodeViewProps {
  project: Project | null;
  onProjectUpdated: () => void;
  pendingSketch: GeneratedSketchResponse | null;
  onPendingSketch: (sketch: GeneratedSketchResponse | null) => void;
}

const KEYWORDS = new Set([
  "void", "int", "long", "unsigned", "float", "double", "char", "bool",
  "boolean", "byte", "short", "const", "static", "volatile", "extern",
  "if", "else", "for", "while", "do", "switch", "case", "break",
  "continue", "return", "default", "struct", "class", "enum", "typedef",
  "sizeof", "true", "false", "HIGH", "LOW", "INPUT", "OUTPUT",
  "INPUT_PULLUP", "LED_BUILTIN", "Serial", "String", "NULL", "nullptr",
]);

function highlightLine(text: string): React.JSX.Element[] {
  const tokens: React.JSX.Element[] = [];
  let i = 0;

  while (i < text.length) {
    // Comments
    if (text.slice(i, i + 2) === "//") {
      tokens.push(
        <span key={i} className="hl-comment">
          {text.slice(i)}
        </span>,
      );
      break;
    }

    // Preprocessor
    if (i === 0 && text.trimStart().startsWith("#")) {
      tokens.push(
        <span key={i} className="hl-preproc">
          {text}
        </span>,
      );
      break;
    }

    // Strings
    if (text[i] === '"') {
      const end = text.indexOf('"', i + 1);
      const strEnd = end === -1 ? text.length : end + 1;
      tokens.push(
        <span key={i} className="hl-string">
          {text.slice(i, strEnd)}
        </span>,
      );
      i = strEnd;
      continue;
    }

    // Single-char literals
    if (text[i] === "'") {
      const end = text.indexOf("'", i + 1);
      const strEnd = end === -1 ? text.length : end + 1;
      tokens.push(
        <span key={i} className="hl-string">
          {text.slice(i, strEnd)}
        </span>,
      );
      i = strEnd;
      continue;
    }

    // Numbers
    if (/\d/.test(text[i]) && (i === 0 || /[\s,;(=+\-*/<>!&|^~%]/.test(text[i - 1]))) {
      let j = i;
      while (j < text.length && /[\d.xXaAbBcCdDeEfFuUlL]/.test(text[j])) j++;
      tokens.push(
        <span key={i} className="hl-number">
          {text.slice(i, j)}
        </span>,
      );
      i = j;
      continue;
    }

    // Identifiers / keywords
    if (/[a-zA-Z_]/.test(text[i])) {
      let j = i;
      while (j < text.length && /[a-zA-Z0-9_]/.test(text[j])) j++;
      const word = text.slice(i, j);
      if (KEYWORDS.has(word)) {
        tokens.push(
          <span key={i} className="hl-keyword">
            {word}
          </span>,
        );
      } else {
        tokens.push(<span key={i}>{word}</span>);
      }
      i = j;
      continue;
    }

    // Plain char
    tokens.push(<span key={i}>{text[i]}</span>);
    i++;
  }

  return tokens;
}

export default function CodeView({
  project,
  onProjectUpdated,
  pendingSketch,
  onPendingSketch,
}: CodeViewProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [flashStatus, setFlashStatus] = useState<FlashStatus>({ state: "idle" });
  const [isEditing, setIsEditing] = useState(false);
  const [editedCode, setEditedCode] = useState<string | null>(null);
  const [fontSize, setFontSize] = useState(13);
  const [generateInstruction, setGenerateInstruction] = useState("");
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleGenerate = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await generateSketch(generateInstruction || undefined);
      onPendingSketch(result);
    } catch (err) {
      const msg = String(err);
      if (msg.toLowerCase().includes("not configured") || msg.toLowerCase().includes("no model")) {
        setError("No code model configured. Go to Settings → Code Model and save a model first.");
      } else if (msg.toLowerCase().includes("connection refused") || msg.toLowerCase().includes("connect")) {
        setError("Cannot reach the model. Make sure Ollama is running (check sidebar status).");
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  }, [onPendingSketch, generateInstruction]);

  const handleUpload = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileSelected = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;
      setLoading(true);
      setError(null);
      try {
        const content = await file.text();
        const result = await uploadSketch(content);
        onPendingSketch(result);
      } catch (err) {
        setError(String(err));
      } finally {
        setLoading(false);
        if (fileInputRef.current) fileInputRef.current.value = "";
      }
    },
    [onPendingSketch],
  );

  const handleApproveAndFlash = useCallback(async () => {
    if (!pendingSketch) return;
    setLoading(true);
    setError(null);
    try {
      await approveSketch(pendingSketch.code);
      onPendingSketch(null);
      onProjectUpdated();

      // Start flash
      setFlashStatus({ state: "compiling" });
      await flashSketch((event: FlashEvent) => {
        switch (event.event) {
          case "compiling":
            setFlashStatus({ state: "compiling" });
            break;
          case "uploading":
            setFlashStatus({ state: "uploading" });
            break;
          case "succeeded":
            setFlashStatus({
              state: "succeeded",
              binarySize: event.data.binary_size,
              maxSize: event.data.max_size,
            });
            break;
          case "failed":
            setFlashStatus({ state: "failed", error: event.data.error });
            break;
        }
      });
    } catch (err) {
      const msg = String(err);
      if (flashStatus.state !== "idle") {
        setFlashStatus({ state: "failed", error: msg });
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  }, [pendingSketch, onPendingSketch, onProjectUpdated, flashStatus.state]);

  const handleApproveOnly = useCallback(async () => {
    if (!pendingSketch) return;
    setLoading(true);
    setError(null);
    try {
      await approveSketch(pendingSketch.code);
      onPendingSketch(null);
      onProjectUpdated();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [pendingSketch, onPendingSketch, onProjectUpdated]);

  const handleFlashCurrent = useCallback(async () => {
    setFlashStatus({ state: "compiling" });
    try {
      await flashSketch((event: FlashEvent) => {
        switch (event.event) {
          case "compiling":
            setFlashStatus({ state: "compiling" });
            break;
          case "uploading":
            setFlashStatus({ state: "uploading" });
            break;
          case "succeeded":
            setFlashStatus({
              state: "succeeded",
              binarySize: event.data.binary_size,
              maxSize: event.data.max_size,
            });
            break;
          case "failed":
            setFlashStatus({ state: "failed", error: event.data.error });
            break;
        }
      });
    } catch (err) {
      setFlashStatus({ state: "failed", error: String(err) });
    }
  }, []);

  const handleReject = useCallback(async () => {
    try {
      await rejectSketch();
    } catch {
      // ignore
    }
    onPendingSketch(null);
  }, [onPendingSketch]);

  if (!project) {
    return (
      <div className="view-placeholder">
        <div className="view-placeholder-title">Code</div>
        <div className="view-placeholder-subtitle">
          Open a project to view and manage Arduino sketches.
        </div>
      </div>
    );
  }

  const currentSketch = project.sketch;
  const hasSketch = !!currentSketch;
  const hasPending = !!pendingSketch;

  // Determine what to display — editing overrides
  const baseCode = hasPending ? pendingSketch.code : currentSketch;
  const displayCode = isEditing && editedCode !== null ? editedCode : baseCode;
  const diffLines = hasPending && !isEditing ? pendingSketch.diff : null;

  const handleZoomIn = () => setFontSize((s) => Math.min(s + 1, 22));
  const handleZoomOut = () => setFontSize((s) => Math.max(s - 1, 9));
  const handleZoomReset = () => setFontSize(13);

  const handleStartEdit = () => {
    setEditedCode(baseCode || "");
    setIsEditing(true);
    setError(null);
  };

  const handleSaveEdit = async () => {
    if (editedCode === null) return;
    setLoading(true);
    try {
      await approveSketch(editedCode);
      onProjectUpdated();
      onPendingSketch(null);
      setIsEditing(false);
      setEditedCode(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditedCode(null);
  };

  // Empty state
  if (!hasSketch && !hasPending) {
    return (
      <div className="code-view">
        <div className="code-empty">
          <GlassPanel tier="standard" className="code-empty-card">
            <div className="code-empty-title">No Sketch Yet</div>
            <div className="code-empty-desc">
              Generate a sketch from your manifest or upload your own .ino file.
            </div>
            {error && <div className="code-error">{error}</div>}
            <textarea
              className="code-gen-instruction-input"
              placeholder="Optional instructions… e.g. use PID for motor control, add watchdog timer, prefer AccelStepper library"
              value={generateInstruction}
              onChange={(e) => setGenerateInstruction(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) handleGenerate();
              }}
              rows={2}
              disabled={loading}
            />
            <div className="code-empty-actions">
              <button
                className="code-generate-btn"
                onClick={handleGenerate}
                disabled={loading}
              >
                {loading ? "Generating…" : "Generate from Manifest"}
              </button>
              <button
                className="code-upload-btn"
                onClick={handleUpload}
                disabled={loading}
              >
                Upload .ino
              </button>
            </div>
          </GlassPanel>
        </div>
        <input
          ref={fileInputRef}
          type="file"
          accept=".ino,.cpp,.c"
          style={{ display: "none" }}
          onChange={handleFileSelected}
        />
      </div>
    );
  }

  return (
    <div className="code-view">
      {/* Flash progress overlay */}
      {flashStatus.state !== "idle" && (
        <div className="flash-overlay">
          <GlassPanel tier="strong" className="flash-modal">
            {flashStatus.state === "compiling" && (
              <>
                <div className="flash-icon flash-icon-amber">◉</div>
                <div className="flash-status">Compiling sketch…</div>
                <div className="flash-bar">
                  <div className="flash-bar-fill flash-bar-pulse" />
                </div>
                <div className="flash-detail">
                  Board: {project?.manifest.board}<br />
                  Port: {project?.manifest.serial_port}
                </div>
              </>
            )}
            {flashStatus.state === "uploading" && (
              <>
                <div className="flash-icon flash-icon-cyan">◉</div>
                <div className="flash-status">Uploading to board…</div>
                <div className="flash-bar">
                  <div className="flash-bar-fill flash-bar-pulse flash-bar-cyan" />
                </div>
                <div className="flash-detail">
                  Port: {project?.manifest.serial_port}
                </div>
              </>
            )}
            {flashStatus.state === "succeeded" && (
              <>
                <div className="flash-icon flash-icon-green">✓</div>
                <div className="flash-status">Flashed successfully</div>
                {flashStatus.binarySize > 0 && (
                  <div className="flash-detail">
                    {flashStatus.binarySize} bytes
                    {flashStatus.maxSize > 0 &&
                      ` (${Math.round((flashStatus.binarySize / flashStatus.maxSize) * 100)}% of ${flashStatus.maxSize})`}
                  </div>
                )}
                <div className="flash-actions">
                  <button
                    className="code-generate-btn runtime-start-btn"
                    onClick={async () => {
                      try {
                        await openRuntimeWindow();
                      } catch (err) {
                        setError(String(err));
                      }
                      setFlashStatus({ state: "idle" });
                    }}
                  >
                    ▶ Start Runtime
                  </button>
                  <button
                    className="code-upload-btn"
                    onClick={() => setFlashStatus({ state: "idle" })}
                  >
                    Dismiss
                  </button>
                </div>
              </>
            )}
            {flashStatus.state === "failed" && (
              <>
                <div className="flash-icon flash-icon-red">✕</div>
                <div className="flash-status">Flash failed</div>
                <pre className="flash-error">{flashStatus.error}</pre>
                <div className="flash-actions">
                  <button
                    className="code-generate-btn"
                    onClick={() => {
                      setFlashStatus({ state: "idle" });
                      handleFlashCurrent();
                    }}
                  >
                    Try Again
                  </button>
                  <button
                    className="code-upload-btn"
                    onClick={() => setFlashStatus({ state: "idle" })}
                  >
                    Dismiss
                  </button>
                </div>
              </>
            )}
          </GlassPanel>
        </div>
      )}

      {/* Approve/Reject bar */}
      {hasPending && (
        <div className="code-action-bar">
          <div className="code-action-label">
            {diffLines ? "Review changes" : "New sketch generated"}
          </div>
          <div className="code-action-buttons">
            <button
              className="code-approve-btn code-flash-btn"
              onClick={handleApproveAndFlash}
              disabled={loading}
            >
              {loading ? "Flashing…" : "Approve & Flash"}
            </button>
            <button
              className="code-approve-btn"
              onClick={handleApproveOnly}
              disabled={loading}
            >
              Save Only
            </button>
            <button
              className="code-reject-btn"
              onClick={handleReject}
              disabled={loading}
            >
              Reject
            </button>
          </div>
        </div>
      )}

      {error && <div className="code-error">{error}</div>}

      {/* Code display */}
      <div className="code-scroll">
        <GlassPanel tier="subtle" className="code-block">
          {/* Zoom + Edit toolbar */}
          <div className="code-block-toolbar">
            <div className="code-zoom-controls">
              <button className="code-zoom-btn" onClick={handleZoomOut} title="Zoom out (−)">−</button>
              <button className="code-zoom-btn code-zoom-reset" onClick={handleZoomReset} title="Reset zoom">{fontSize}px</button>
              <button className="code-zoom-btn" onClick={handleZoomIn} title="Zoom in (+)">+</button>
            </div>
            {!hasPending && !isEditing && (
              <button className="code-edit-btn" onClick={handleStartEdit}>
                ✎ Edit
              </button>
            )}
            {isEditing && (
              <div className="code-edit-actions">
                <button className="code-edit-save-btn" onClick={handleSaveEdit} disabled={loading}>
                  {loading ? "Saving…" : "Save"}
                </button>
                <button className="code-edit-cancel-btn" onClick={handleCancelEdit}>
                  Cancel
                </button>
              </div>
            )}
          </div>

          {isEditing ? (
            <textarea
              className="code-editor-textarea"
              style={{ fontSize }}
              value={editedCode ?? ""}
              onChange={(e) => setEditedCode(e.target.value)}
              spellCheck={false}
            />
          ) : diffLines ? (
            renderDiffView(diffLines, fontSize)
          ) : (
            renderPlainView(displayCode || "", fontSize)
          )}
        </GlassPanel>
      </div>

      {/* Flash to Board banner — shown when tools are compiled and no pending diff */}
      {!hasPending && project.has_tools && !isEditing && (
        <div className="code-flash-ready-bar">
          <div className="code-flash-ready-info">
            <span className="code-flash-ready-dot" />
            <span className="code-flash-ready-label">Tools compiled</span>
            <span className="code-flash-ready-sub">
              {project.manifest.board} · {project.manifest.serial_port}
            </span>
          </div>
          <button
            className="code-flash-ready-btn"
            onClick={handleFlashCurrent}
            disabled={loading || flashStatus.state !== "idle"}
          >
            ⚡ Flash to Board
          </button>
        </div>
      )}

      {/* Footer actions for existing sketches */}
      {!hasPending && (
        <div className="code-footer">
          <textarea
            className="code-gen-instruction-input code-gen-instruction-footer"
            placeholder="Optional instructions for regenerate… (⌘↵ to run)"
            value={generateInstruction}
            onChange={(e) => setGenerateInstruction(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) handleGenerate();
            }}
            rows={1}
            disabled={loading}
          />
          <div className="code-footer-actions">
            {!project.has_tools && (
              <button
                className="code-generate-btn code-small-btn code-flash-btn"
                onClick={handleFlashCurrent}
                disabled={loading || flashStatus.state !== "idle"}
              >
                ⚡ Flash
              </button>
            )}
            <button
              className="code-generate-btn code-small-btn"
              onClick={handleGenerate}
              disabled={loading}
            >
              {loading ? "Generating…" : "Regenerate"}
            </button>
            <button
              className="code-upload-btn code-small-btn"
              onClick={handleUpload}
              disabled={loading}
            >
              Upload .ino
            </button>
          </div>
        </div>
      )}

      <input
        ref={fileInputRef}
        type="file"
        accept=".ino,.cpp,.c"
        style={{ display: "none" }}
        onChange={handleFileSelected}
      />
    </div>
  );
}

function renderPlainView(code: string, fontSize = 13) {
  const lines = code.split("\n");
  return (
    <pre className="code-lines" style={{ fontSize }}>
      {lines.map((line, idx) => (
        <div key={idx} className="code-line">
          <span className="code-line-num">{idx + 1}</span>
          <span className="code-line-content">{highlightLine(line)}</span>
        </div>
      ))}
    </pre>
  );
}

function renderDiffView(diff: DiffLine[], fontSize = 13) {
  return (
    <pre className="code-lines" style={{ fontSize }}>
      {diff.map((line, idx) => {
        const statusClass =
          line.status === "added"
            ? "code-line-added"
            : line.status === "removed"
              ? "code-line-removed"
              : "";
        const marker =
          line.status === "added"
            ? "+"
            : line.status === "removed"
              ? "−"
              : " ";

        return (
          <div key={idx} className={`code-line ${statusClass}`}>
            <span className="code-line-marker">{marker}</span>
            <span className="code-line-num">{line.line_number}</span>
            <span className="code-line-content">{highlightLine(line.content)}</span>
          </div>
        );
      })}
    </pre>
  );
}
