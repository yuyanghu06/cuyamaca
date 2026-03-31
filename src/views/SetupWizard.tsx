import { useState, useEffect, useCallback } from "react";
import DependencyRow from "../components/DependencyRow";
import {
  checkDependencies,
  skipDependencySetup,
  markSetupComplete,
} from "../commands/setup";
import type { DepState } from "../commands/setup";

interface Props {
  onComplete: () => void;
}

export default function SetupWizard({ onComplete }: Props) {
  const [ollamaState, setOllamaState] = useState<DepState>({
    state: "installing",
    data: { progress: 0, message: "Checking..." },
  });
  const [arduinoState, setArduinoState] = useState<DepState>({
    state: "installing",
    data: { progress: 0, message: "Checking..." },
  });
  const [checked, setChecked] = useState(false);

  useEffect(() => {
    checkDependencies()
      .then((status) => {
        setOllamaState(status.ollama);
        setArduinoState(status.arduinoCli);
        setChecked(true);
      })
      .catch(() => {
        setOllamaState({ state: "missing" });
        setArduinoState({ state: "missing" });
        setChecked(true);
      });
  }, []);

  const allReady =
    checked &&
    ollamaState.state === "ready" &&
    arduinoState.state === "ready";

  const anyInstalling =
    ollamaState.state === "installing" ||
    arduinoState.state === "installing";

  const handleContinue = useCallback(async () => {
    try {
      await markSetupComplete();
    } catch {
      // Non-fatal
    }
    onComplete();
  }, [onComplete]);

  const handleSkip = useCallback(async () => {
    try {
      await skipDependencySetup();
    } catch {
      // Non-fatal
    }
    onComplete();
  }, [onComplete]);

  return (
    <div className="setup-wizard">
      <div className="setup-wizard-inner">
        <div className="setup-header">
          <div className="setup-icon">◈</div>
          <h1 className="setup-title">Welcome to Cuyamaca</h1>
          <p className="setup-subtitle">
            Let's make sure everything is set up for natural language Arduino
            control.
          </p>
        </div>

        <div className="setup-deps">
          <DependencyRow
            name="ollama"
            label="Ollama"
            state={ollamaState}
            onStateChange={setOllamaState}
          />
          <DependencyRow
            name="arduino-cli"
            label="arduino-cli"
            state={arduinoState}
            onStateChange={setArduinoState}
          />
        </div>

        <div className="setup-actions">
          {allReady ? (
            <button className="setup-continue-btn" onClick={handleContinue}>
              Continue
            </button>
          ) : (
            <button
              className="setup-continue-btn"
              disabled={anyInstalling || !checked}
              onClick={handleContinue}
              title={
                !checked
                  ? "Checking dependencies..."
                  : anyInstalling
                    ? "Installation in progress..."
                    : "Some dependencies are missing"
              }
            >
              {!checked
                ? "Checking..."
                : anyInstalling
                  ? "Installing..."
                  : "Continue anyway"}
            </button>
          )}
          <button
            className="setup-skip-btn"
            onClick={handleSkip}
            disabled={anyInstalling}
          >
            Skip — I'll set up manually
          </button>
        </div>
      </div>
    </div>
  );
}
