import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './App.css';

type Phase = 'idle' | 'capturing' | 'generating' | 'playing' | 'paused' | 'stopped' | 'error';

interface StatusDto {
  phase: Phase;
  hotkey_error?: string | null;
}

interface StateEventPayload {
  type: 'phase-changed' | 'progress' | 'error';
  to?: Phase;
  message?: string;
}

interface AppConfig {
  voice_fr: string;
  voice_en: string;
  speech_rate: number;
  hotkey_read: string;
  hotkey_pause: string;
  hotkey_stop: string;
}

interface Voices {
  fr: string[];
  en: string[];
}

const PHASE_LABEL: Record<Phase, string> = {
  idle: 'Inactif',
  capturing: 'Capture…',
  generating: 'Génération…',
  playing: 'Lecture',
  paused: 'Pause',
  stopped: 'Arrêté',
  error: 'Erreur',
};

function App() {
  const [phase, setPhase] = useState<Phase>('idle');
  const [hotkeyError, setHotkeyError] = useState<string | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [voices, setVoices] = useState<Voices>({ fr: [], en: [] });

  async function run(action: () => Promise<unknown>) {
    setLastError(null);
    try {
      await action();
    } catch (err) {
      setLastError(String(err));
    }
  }

  const handlePauseResume = () =>
    run(() => invoke(phase === 'paused' ? 'resume_reading' : 'pause_reading'));

  // Relit la configuration avant d'écrire : le panneau ne doit jamais écraser un réglage
  // modifié entre-temps dans la fenêtre Paramètres.
  const updateConfig = (patch: Partial<AppConfig>) =>
    run(async () => {
      const current = await invoke<AppConfig>('get_config');
      const next = { ...current, ...patch };
      await invoke('set_config', { config: next });
      setConfig(next);
    });

  useEffect(() => {
    // Préchargement du modèle Kokoro au lancement (non bloquant, idempotent).
    invoke('load_model').catch(() => {});
    invoke<Voices>('get_voices').then(setVoices);

    const loadAll = () => {
      invoke<StatusDto>('get_status').then((dto) => {
        setPhase(dto.phase);
        setHotkeyError(dto.hotkey_error ?? null);
      });
      invoke<AppConfig>('get_config').then(setConfig);
    };
    loadAll();
    window.addEventListener('focus', loadAll);

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') invoke('hide_panel');
    };
    window.addEventListener('keydown', onKey);

    const unlisten = listen<StateEventPayload>('navi-state', (event) => {
      const payload = event.payload;
      if (payload.type === 'phase-changed' && payload.to) setPhase(payload.to);
      else if (payload.type === 'error' && payload.message) setLastError(payload.message);
    });

    return () => {
      window.removeEventListener('focus', loadAll);
      window.removeEventListener('keydown', onKey);
      unlisten.then((fn) => fn());
    };
  }, []);

  const playing = phase === 'playing' || phase === 'paused';

  return (
    <main className="panel">
      <header>
        <h1>Navi</h1>
        <span className={`phase-badge phase-${phase}`}>{PHASE_LABEL[phase]}</span>
      </header>

      {lastError && <p className="error-box">{lastError}</p>}
      {hotkeyError && <p className="error-box">⚠️ {hotkeyError}</p>}

      <div className="action-row">
        <button onClick={handlePauseResume} disabled={!playing}>
          {phase === 'paused' ? 'Reprendre' : 'Pause'}
        </button>
        <button
          onClick={() => run(() => invoke('stop_reading'))}
          disabled={phase === 'idle' || phase === 'stopped'}
        >
          Arrêt
        </button>
      </div>

      {config && (
        <section className="settings">
          <label>
            Voix française
            <select
              value={config.voice_fr}
              onChange={(e) => updateConfig({ voice_fr: e.target.value })}
            >
              {voices.fr.map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </label>
          <label>
            Voix anglaise
            <select
              value={config.voice_en}
              onChange={(e) => updateConfig({ voice_en: e.target.value })}
            >
              {voices.en.map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </label>
          <label>
            Vitesse ×{config.speech_rate.toFixed(1)}
            <input
              type="range"
              min="0.5"
              max="2"
              step="0.1"
              value={config.speech_rate}
              onChange={(e) => setConfig({ ...config, speech_rate: Number(e.target.value) })}
              onPointerUp={() => updateConfig({ speech_rate: config.speech_rate })}
              onKeyUp={() => updateConfig({ speech_rate: config.speech_rate })}
            />
          </label>
          <p className="shortcuts">
            Lire : {config.hotkey_read} · Pause : {config.hotkey_pause} · Arrêt:{' '}
            {config.hotkey_stop}
          </p>
        </section>
      )}

      <footer>
        <button onClick={() => run(() => invoke('open_settings'))}>Paramètres</button>
        <button onClick={() => invoke('quit_app')}>Quitter</button>
      </footer>
    </main>
  );
}

export default App;
