# S1-083 — Signature de démarrage : enregistrement du propriétaire

Date : 2026-11-20. Carte : N-083 (P2, S). Branche : `fix/TW-078-audio-silence` (livraison demandée dans la PR #87).

## Demande

Le propriétaire a déposé son propre enregistrement `hey_listen.mp3` à la racine du projet et souhaite
qu'il soit joué à l'ouverture de Navi, à la place de la signature produite par TTS (carte N-056).

## Mise en œuvre

| Élément | Avant | Après |
|---|---|---|
| Fichier embarqué | `src-tauri/resources/hey_listen.wav` (TTS) | `src-tauri/resources/hey_listen.mp3` (enregistrement) |
| Résolution | `utils::resolve_signature_wav()` | `utils::resolve_signature()` |
| Variable d'override | `NAVI_SIGNATURE_WAV` | `NAVI_SIGNATURE_FILE` (l'ancienne reste acceptée) |
| Bundle Tauri | `resources/hey_listen.wav` | `resources/hey_listen.mp3` |
| `.gitattributes` | — | `*.mp3 binary` |

Le décodage passe par rodio/symphonia, déjà compilé avec le support MP3 : **aucun transcodage, aucune
dépendance ajoutée**. La signature est jouée une seule fois, après que les raccourcis et la capture
sont prêts (carte N-057), dans un thread dédié — le démarrage n'est jamais bloqué.

## Vérification

- `signature_de_demarrage_presente_et_decodable` : la ressource est ouverte et **décodée**
  (MP3 → échantillons > 0), sans périphérique audio.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- Au lancement, la trace affiche `[navi] signature de démarrage : …\resources\hey_listen.mp3`
  et aucune erreur de décodage.

## Traçabilité

Carte N-083 ; décision consignée dans `docs/decisions.md` (« la signature de démarrage devient
l'enregistrement du propriétaire »). Remplace la forme finale décidée en N-055/N-056.
