import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type Phase =
  | "idle"
  | "capturing"
  | "generating"
  | "playing"
  | "paused"
  | "stopped"
  | "error";

interface StatusDto {
  phase: Phase;
  current_chunk: number;
  total_chunks: number;
  engine_loaded: boolean;
  audio_ready: boolean;
  hotkey_error?: string | null;
}

interface StateEventPayload {
  type: "phase-changed" | "progress" | "error";
  from?: Phase;
  to?: Phase;
  current?: number;
  total?: number;
  message?: string;
}

const PHASE_LABEL: Record<Phase, string> = {
  idle: "Inactif",
  capturing: "Capture…",
  generating: "Génération…",
  playing: "Lecture",
  paused: "Pause",
  stopped: "Arrêté",
  error: "Erreur",
};

function App() {
  const [status, setStatus] = useState<StatusDto>({
    phase: "idle",
    current_chunk: 0,
    total_chunks: 0,
    engine_loaded: false,
    audio_ready: false,
    hotkey_error: null,
  });
  const [lastError, setLastError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function refreshStatus() {
    const dto = await invoke<StatusDto>("get_status");
    setStatus(dto);
  }

  async function handleRead() {
    setBusy(true);
    setLastError(null);
    try {
      await invoke("start_reading");
      await refreshStatus();
    } catch (err) {
      setLastError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handlePauseResume() {
    setLastError(null);
    try {
      if (status.phase === "paused") {
        await invoke("resume_reading");
      } else {
        await invoke("pause_reading");
      }
      await refreshStatus();
    } catch (err) {
      setLastError(String(err));
    }
  }

  async function handleStop() {
    setLastError(null);
    try {
      await invoke("stop_reading");
      await refreshStatus();
    } catch (err) {
      setLastError(String(err));
    }
  }

  async function handlePreload() {
    setLastError(null);
    try {
      await invoke("load_model");
      await refreshStatus();
    } catch (err) {
      setLastError(String(err));
    }
  }

  useEffect(() => {
    // Préchargement du modèle Kokoro au lancement (non bloquant, idempotent).
    invoke("load_model").catch(() => {
      /* le message d'erreur est remontée via l'UI si besoin */
    });

    let mounted = true;

    invoke<StatusDto>("get_status").then((dto) => {
      if (mounted) setStatus(dto);
    });

    const unlisten = listen<StateEventPayload>("navi-state", (event) => {
      const payload = event.payload;
      if (payload.type === "phase-changed" && payload.to) {
        setStatus((s) => ({ ...s, phase: payload.to! }));
      } else if (payload.type === "progress") {
        setStatus((s) => ({
          ...s,
          current_chunk: payload.current ?? s.current_chunk,
          total_chunks: payload.total ?? s.total_chunks,
        }));
      } else if (payload.type === "error" && payload.message) {
        setLastError(payload.message);
      }
    });

    return () => {
      mounted = false;
      unlisten.then((fn) => fn());
    };
  }, []);

  const progressPercent =
    status.total_chunks > 0 ? Math.round((status.current_chunk / status.total_chunks) * 100) : 0;

  return (
    <main className="container">
      <h1>Navi</h1>
      <p className="subtitle">
        Lecteur local — {status.engine_loaded ? "modèle chargé" : "modèle non chargé"} · {status.audio_ready ? "audio prêt" : "audio non prêt"}
      </p>

      <div className="status-row">
        <span className={`phase-badge phase-${status.phase}`}>{PHASE_LABEL[status.phase]}</span>
        {status.total_chunks > 0 && (
          <span className="progress-text">
            {status.current_chunk}/{status.total_chunks} segments ({progressPercent}%)
          </span>
        )}
      </div>

      <progress max={Math.max(status.total_chunks, 1)} value={status.current_chunk} />

      {lastError && <p className="error-box">{lastError}</p>}

      {status.hotkey_error && (
        <p className="error-box" title="Un raccourci n'a pas pu être enregistré">
          ⚠️ {status.hotkey_error}
        </p>
      )}

      <div className="action-row">
        <button
          onClick={handleRead}
          disabled={busy || status.phase === "capturing" || status.phase === "generating" || status.phase === "playing"}
        >
          {busy ? "Capture…" : "Lire la sélection"}
        </button>
        <button onClick={handlePauseResume} disabled={status.phase !== "playing" && status.phase !== "paused"}>
          {status.phase === "paused" ? "Reprendre" : "Pause"}
        </button>
        <button onClick={handleStop} disabled={status.phase === "idle" || status.phase === "stopped"}>
          Arrêt
        </button>
        <button onClick={handlePreload} disabled={status.engine_loaded}>
          {status.engine_loaded ? "Modèle chargé ✓" : "Charger le modèle"}
        </button>
      </div>

      <p className="note">
        La lecture capture le texte sélectionné dans la fenêtre au premier plan (Ctrl+C simulé),
        le nettoie, le segmente et synthétise avec Kokoro, puis joue le résultat.
      </p>
    </main>
  );
}

export default App;
