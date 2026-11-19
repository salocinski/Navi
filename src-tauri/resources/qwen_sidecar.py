#!/usr/bin/env python3
"""Sidecar Qwen3-TTS-12Hz-0.6B-CustomVoice — carte N-071 (migration TTS Navi).

Processus Python persistant : charge le modèle **une fois** (~2,3 Go), puis sert
les requêtes d'un hôte (Navi) en JSONL sur stdin/stdout.

Protocole (une ligne JSON par message, UTF-8, terminée par \\n) :

  → {"id": 1, "text": "bonjour", "language": "French", "speaker": "Ryan"}
  ← {"id": 1, "ok": true, "sample_rate": 24000, "pcm_f32_base64": "...", "n_samples": 123, "duration_s": 5.1}
  ← {"id": 1, "ok": false, "error": "message clair"}

Au démarrage, le sidecar émet une ligne de disponibilité :
  {"ready": true,  "sample_rate": 24000, "device": "cuda:0" | "cpu", "model": "Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice"}
  {"ready": false, "error": "..."}   puis quitte avec le code 1.

Langue : « French » | « English » (tout libellé non-anglais → English).
Voix : speaker CustomVoice (« Ryan » par défaut — voix officielle Navi, carte N-070).
Vitesse : non supportée par qwen3-tts 0.6B (ignorée).
"""
import base64
import json
import sys

MODEL_ID = "Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice"
SAMPLE_RATE = 24000


def emit(obj):
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def log(*args):
    print("[qwen-sidecar]", *args, file=sys.stderr, flush=True)


def load_model():
    import torch
    from qwen_tts import Qwen3TTSModel

    cuda = bool(torch.cuda.is_available())
    if cuda:
        kwargs = dict(device_map="cuda:0", dtype=torch.bfloat16)
    else:
        # CPU : bf16 mal supporté → float32 (plus lent mais fiable).
        kwargs = dict(device_map="cpu", dtype=torch.float32)
    log("chargement du modèle", MODEL_ID, "sur", "cuda:0" if cuda else "cpu", "…")
    model = Qwen3TTSModel.from_pretrained(MODEL_ID, attn_implementation="eager", **kwargs)
    log("modèle chargé")
    return model, ("cuda:0" if cuda else "cpu")


def synthesize(model, req):
    import numpy as np

    text = str(req.get("text") or "").strip()
    if not text:
        raise ValueError("texte vide")
    language = "French" if str(req.get("language") or "French").lower().startswith("fr") else "English"
    speaker = str(req.get("speaker") or "Ryan")
    wavs, sr = model.generate_custom_voice(text=text, language=language, speaker=speaker)
    samples = np.asarray(wavs[0], dtype=np.float32)
    return samples, int(sr)


def main():
    try:
        model, device = load_model()
    except Exception as exc:  # torch absent, modèle introuvable/corrompu, VRAM…
        emit({"ready": False, "error": f"chargement du modèle impossible : {exc}"})
        return 1

    emit({"ready": True, "sample_rate": SAMPLE_RATE, "device": device, "model": MODEL_ID})

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError as exc:
            emit({"id": None, "ok": False, "error": f"requête JSON invalide : {exc}"})
            continue
        rid = req.get("id")
        try:
            samples, sr = synthesize(model, req)
            b64 = base64.b64encode(samples.tobytes()).decode("ascii")
            emit({
                "id": rid,
                "ok": True,
                "sample_rate": sr,
                "pcm_f32_base64": b64,
                "n_samples": int(samples.shape[0]),
                "duration_s": float(samples.shape[0]) / float(sr),
            })
        except Exception as exc:
            emit({"id": rid, "ok": False, "error": str(exc)})
    return 0


if __name__ == "__main__":
    sys.exit(main())
