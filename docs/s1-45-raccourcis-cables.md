# S1-045 — Raccourcis globaux câblés au pipeline (carte TW-045)

Date : 2026-11-19. Carte : N-045 (P1, M). Modules : `src-tauri/src/hotkey/mod.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`.
Critère de la carte : « Échec d'enregistrement signalé à l'utilisateur ».

## Mapping retenu (après mesure sur machine de référence — TW-018)

| Raccourci | Action | Portée |
|---|---|---|
| **Ctrl+Maj+Espace** | Lire la sélection | capture → voix haute (le raccourci déclenche la même chaîne que le bouton « Lire la sélection ») |
| **Ctrl+Maj+P** | Pause ↔ Reprise | bascule Lecture ↔ Pause selon l'état de la machine d'états |
| **Ctrl+Maj+S** | Arrêt | arrêt complet (file vidée, génération annulée) |

Cadeau du TW-018 : Ctrl+Maj+Espace est libre, contrairement à Ctrl+Alt+Espace.

## Ce qui a été fait

### 1. Câblage effectif `main → app → machine d'états`
- `setup()` enregistre désormais les trois raccourcis via `HotkeyManager::register_defaults()`.
- Un **thread d'écoute global** consomme `GlobalHotKeyEvent::receiver()`.
  - `ReadSelection` → `read_selection_impl` (la même fonction que la commande IPC `start_reading`).
  - `PauseResume` → **toggle** : `Paused → resume`, sinon `pause`.
  - `Stop` → `stop_reading_impl`.
- Le manager Windows vit le temps de l'app (`mem::forget`) — on ne drop `GlobalHotKeyManager` car le `WindowClass` et le dispatch des `WM_HOTKEY` doivent rester vivants ; la boucle Tauri pompe les messages main-thread.

### 2. Rapport d'enregistrement → UI
- `NaviState::hotkey_report: Mutex<RegistrationReport>` contient le résultat de `register_defaults()`.
- `get_status` expose `hotkey_error: Option<String>` (sortie de `RegistrationReport::failure_message()`).
- L'UI affiche une boîte d'erreur rouge avec l'icône ⚠️ si au moins un raccourci est tombé ; aucun raccourci n'est requis — l'app démarre et fonctionne normalement sans.
- Si `GlobalHotKeyManager::new()` échoue (pas de window message, mauvais OS) : message dans stderr, app démarre quand même.

### 3. Refactor compatibilité
`lib.rs` a été réécrit avec un type `NaviRef` (vue clonable de l'état) : les mêmes fonctions `read_selection_impl`/`pause_reading_impl`/`resume_reading_impl`/`stop_reading_impl` sont appelées **par les commandes Tauri et par le thread d'écoute** — aucune duplication.

## Limites documentées

1. **Pas de feedback audible** pour un échec de raccourci. Le message d'erreur est visible dans le panneau (⚠️) ; à vaquer soit à une signature sonore (TW-055) soit à un texte inline dans la zone ISO — non bloquant.
2. **Ctrl+Maj+Espace peut être pris** par une autre application sur la machine de l'humain (comme Ctrl+Alt+Espace l'était vers TW-018) ; le rapport `failure_message()` le signale, mais cette boîte n'émet que des erreurs d'enregistrement, pas d'avertissement persistant.
3. **Aucun raccourci n'agit sur l'UI elle-même** (focus panneau, etc.) — seulement sur la machine d'états/cœur d'app.

## Vérification

```
cargo test     → 88 passed, 0 failed (unit+intégration)
npm run build  → OK, hotkey_error inclus dans StatusDto
```

Outil de diagnostic : `cargo run --example hotkey_check` (vérifie chaque combinaison candidats).

## Traçabilité

- Carte : N-045. Sources : `lib.rs`, `hotkey/mod.rs`, `App.tsx`, `docs/s1-18-raccourcis-conflits.md`.
- Critère de la carte : « Échec d'enregistrement signalé à l'utilisateur » → ✓ via `get_status.hotkey_error` + box UI.
