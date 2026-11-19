# S1-062 — Tests et CI (carte N-062)

Date : 2026-11-19. Carte : N-062 (P1, M). Branche : `feat/TW-062-tests-ci`.

## État de départ (mesuré, pas supposé)

- **Rust** : 88 tests déjà présents et verts (70 unitaires — capture, machine d'états, audio/file, texte, config, tts/engine, utils — + 3 intégration `lecture_cycle` + 15 intégration `text_pipeline`). `kokoro_smoke` ignoré par défaut (exige les modèles, machine de dev uniquement). La couverture cible de la carte (capture, file audio, machine d'états) existait ; la carte consistait à la **faire tenir par la CI et le lint**, pas à la recréer.
- **CI** : aucune (pas de `.github/`).
- **Lint Rust** : 152 hunks non conformés `rustfmt` ; 27 warnings `clippy` (19 auto-corrigeables, 8 manuels).
- **Lint TS** : aucune config ; `prettier`/`eslint` décidés au 2026-11-17 (voir `docs/decisions.md` — Linting et formatage) mais jamais appliqués.

## Ce qui a été fait

### Rust

- `cargo fmt` sur tout l'arbre (152 hunks, purement mécanique — aucune logique).
- `cargo clippy --fix --all-targets` (19 corrections) + corrections manuelles :
  - `use std::sync::{Arc, Mutex}` de `pipeline.rs` ré-ajouté sous `#[cfg(test)]` (le fix l'avait retiré ; utilisé uniquement par `MemorySink`, cfg(test)) ;
  - `init` (champ jamais relu) retiré de `Phonemizer` ; `espeak_Initialize` reste appelé en local dans le constructeur ;
  - `static HOTKEY_MANAGER` (jamais utilisé) retiré, ainsi que `use once_cell::sync::Lazy;` et la dépendance directe `once_cell` ;
  - `let segments = segments;` (shadowing redondant) retiré ; ifs collapsés, `RangeInclusive::contains`, `MemorySink::default()`, `play_signature_once` déplacée avant `mod tests`.
- Résultat : **`cargo clippy --all-targets -- -D warnings` passe** ; **`cargo fmt --check` passe** ; **88 tests verts**.

### TypeScript / frontend

- `prettier` + `eslint@9` (flat config) + `typescript-eslint` + `eslint-plugin-react-hooks`.
- Scripts npm : `lint`, `format`, `format:check`.
- Sources normalisées (guillemets simples, largeur 100) ; `npm run lint` et `npm run format:check` passent ; `npm run build` (tsc + vite) passe.
- Exclusions **explicites** (`.prettierignore`) : `docs/`, `BACKLOG.md`, `README.md` (docs ATLAS/français maintenus à la main) ; `index.html`, `tsconfig.json`, `tsconfig.node.json` (fichiers CRLF hérités, `core.autocrlf=true` — non normalisés pour éviter le flip-flop LF/CRLF) ; `src-tauri/`, `kanban.html`, `.atlas/`, `scratch/`.

### CI — `.github/workflows/ci.yml` (windows-latest)

Étapes : `npm ci` → `format:check` → `eslint` → `npm run build` → `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test`.

- `windows-latest` : le crate `windows` (Win32 SendInput/presse-papiers) impose Windows pour la compilation.
- `dtolnay/rust-toolchain@stable` (rustfmt + clippy) + `Swatinem/rust-cache@v2` (workspaces `src-tauri`).
- `kokoro_smoke` reste ignoré : aucun modèle sur le runner (`models/` gitignoré) ; la fumée Kokoro reste une recette humaine locale.

## Limites et hors-périmètre (assumé)

1. **Aucun test audio réel en CI** : aucun périphérique sur runner ; la validation audible reste humaine (N-020/N-021/N-041).
2. **`format:check` ne couvre pas** `docs/`, `BACKLOG.md`, `README.md` (exclus) : le reflux prettier de ~1500 lignes sur les docs a été écarté pour la relectabilité du PR ; la décision est tracée ici et dans `.prettierignore`.
3. **Les 3 fichiers CRLF exclus** restent non vérifiés par prettier.
4. La couverture **n'a pas été étendue** : capture/file d'attente/machine d'états étaient déjà testées ; rien ne garantit aujourd'hui la capture réelle hors Chromium/Word (voir N-016, rapport partiel, limitation tracée).

## Vérification

```text
cargo fmt --check                                    → OK
cargo clippy --all-targets -- -D warnings            → OK (0 warning)
cargo test                                           → 70 + 3 + 15 verts (kokoro_smoke ignoré)
npm run lint / format:check / build                  → OK
npx prettier --check .github                         → OK
```
