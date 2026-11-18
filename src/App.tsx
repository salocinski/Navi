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
  const [status, setStatus] = useState<StatusDto>({ phase: "idle", current_chunk: 0, total_chunks: 0 });
  const [lastError, setLastError] = useState<string | null>(null);

  useEffect(() => {
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
      <p className="subtitle">Lecteur local — contrat machine d'états → UI (N-032)</p>

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

      <div className="action-row">
        <button disabled title="Branché par N-041/N-045 (moteur et raccourcis)">Lire</button>
        <button disabled title="Branché par N-041/N-045 (moteur et raccourcis)">Pause</button>
        <button disabled title="Branché par N-041/N-045 (moteur et raccourcis)">Arrêt</button>
      </div>

      <p className="note">
        Les commandes de lecture sont affichées en lecture seule : la capture, le moteur Kokoro et
        les raccourcis seront branchés par N-039/N-041/N-045.
      </p>
    </main>
  );
}

export default App;
