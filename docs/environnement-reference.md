# Environnement de référence - Navi (carte TW-003, S0-00c)

Date : 2026-11-17. Statut : **documenté**.

## Objectif

Figer l'environnement matériel et logiciel de référence utilisé pour les
expériences Sprint 0 (benchmarks, décisions go/no-go) et la reproductibilité
des résultats. Vérifier qu'aucune dépendance à CUDA Toolkit n'est supposée.

## Environnement de référence cible (d'après le backlog)

| Composant | Spécification |
|---|---|
| OS | Windows 11 |
| CPU | Intel Core i7-7700K @ 4,2 GHz |
| RAM | 32 Go |
| GPU | NVIDIA GeForce RTX 3060 Ti 8 Go |

## Machine de développement et de mesure (constatée)

| Composant | Valeur mesurée |
|---|---|
| OS | **Windows 10 Home 10.0.19045 (build 19045), 64 bits** |
| Carte mère | MSI MS-7A71 |
| CPU | Intel Core i7-7700K @ 4,20 GHz, 4 cœurs / 8 threads |
| RAM | 32 Go (31,9 Go utilisables) |
| GPU intégré | Intel HD Graphics 630 (1 Go) |
| GPU dédié | NVIDIA GeForce RTX 3060 Ti, 8 Go (8192 Mio), driver **596.49** |
| Rust | 1.99.1 (2026-10-13) |
| Cargo | 1.99.1 (2026-08-27) |
| Node.js | v26.4.0 |
| npm | 11.17.0 |

## Divergence à signaler (R9)

L'environnement cible du backlog (**Windows 11**) diffère de la machine de
développement/mesure actuelle (**Windows 10 Home 10.0.19045**) : même CPU,
même RAM, même GPU, OS en retard d'une version majeure.

- Impact attendu sur le Sprint 0 : faible. La chaîne TTS visée (Kokoro via ONNX
  Runtime, EP CPU, `ort` en `load-dynamic`) ne dépend pas de fonctionnalités
  exclusives à Windows 11.
- Décision : les mesures Sprint 0 sont faites sur cette machine, et le
  document de synthèse doit indiquer « mesuré sur Windows 10 Home ». La cible
  produit reste Windows 11 ; tout résultat en décalage devra être re-validé
  sur Win 11 avant livraison finale.
- Action humaine suggérée : confirmer ou non que la machine de référence
  officielle devient celle-ci, ou prévoir une machine Win 11 pour la clôture.

## Vérification : aucune dépendance CUDA Toolkit supposée

Constats mesurés sur la machine :

- `nvcc` absent du PATH ; `CUDA_PATH` vide ; le dossier
  `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA` n'existe pas.
- Driver NVIDIA présent (driver 596.49) : le GPU est utilisable via les SDK
  driver, pas via CUDA Toolkit.

Constats dans le dépôt :

- `src-tauri/Cargo.toml` : aucune mention de `cuda`, `cudnn`, `cublas`,
  `tensorrt`, `nvidia` ou `directml`. Dépendances : `ort 2.0.0-rc.15` en
  `load-dynamic` (onnxruntime chargé dynamiquement à l'exécution, EP CPU par
  défaut), `rodio 0.19`, `global-hotkey 0.3`, `windows 0.58`.
- `src-tauri/Cargo.lock` : aucune occurrence `cuda` / `cudnn` / `tensorrt` /
  `nvidia`.
- `package.json` : aucun paquet lié à CUDA/DirectML.
- Les seules occurrences du mot « CUDA » dans le projet sont documentaires
  (BACKLOG.md, kanban, decisions.md), jamais dans des dépendances.

**Vérification : satisfaite. Aucune dépendance à CUDA Toolkit n'est supposée,
ni installée ni référencée.**

## DirectML / CUDA et accélération GPU

La chaîne ONNX Runtime du projet tourne sur EP CPU. Le GPU sera testé
uniquement sur la carte GPU (S0-02c, TW-8), DirectML de préférence (présent
avec le driver NVIDIA), CUDA Toolkit non requis. Toute tentative GPU
doit être documentée avec les conditions de reproductibilité (OS, driver, onnxruntime EP).

## Contraintes et remarques pour les benchmarks

- La machine n'est pas une VM et n'est pas en laptop (desktop MSI MS-7A71) ;
  pas de throttling thermique tiers à prendre en compte, mais le i7-7700K
  reste un quad-cœur : les RTF CPU mesurés seront des résultats « machine de
  référence », pas des chiffres universels.
- La GPU intégrée HD 630 doit être neutralisée dans tout protocole de
  benchmark (forcer la sélection du GPU dédié ou mesurer CPU uniquement).
- Vérifier lors des benchmarks que le driver NVIDIA 596.49 reste compatible
  avec ONNX Runtime DirectML, sans CUDA Toolkit.
- Toute recréation de machine de référence devra au minimum reprendre :
  i7-7700K, 32 Go, RTX 3060 Ti, Windows (10 ou 11 - voir divergence), driver
  NVIDIA ≥ 596.49.

## Références

- BACKLOG.md - carte TW-003 / **S0-00c** (source : `Navi_Synthese_Projet_Sprint_0.docx`).
- decisions.md - entrée « Environnement CPU contre DirectML pour ONNX Runtime ».
- TW-008 / S0-02c - accélération GPU si la chaîne est disponible.
