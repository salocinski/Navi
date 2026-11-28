# Procédure de release

Pas de CI de publication : la release est construite localement puis publiée à la main.

1. Versions alignées dans `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` ; entrée dans `CHANGELOG.md`.
2. Environnement : CUDA 13.5, Visual Studio (voir `docs/environnement-reference.md`), `CMAKE_CUDA_ARCHITECTURES="75;86;89"` (ne pas laisser `86` seul), `CUDA_PATH`/`CUDA_PATH_V13_5` et `BINDGEN_EXTRA_CLANG_ARGS` (voir `docs/s3-90-whisper-build.md`) : sans eux, le build échoue sur `whisper-rs-sys`. Valeurs et chemins du poste de référence : section « Environnement de build » ci-dessous.
3. `node tools/copy-cuda-dlls.mjs` puis `npm run bundle:win` → `src-tauri/target/release/bundle/nsis/Navi_<version>_x64-setup.exe`.
4. Contrôler l'exécutable : manifeste `requireAdministrator`, DLL `cublas64_13.dll`/`cublasLt64_13.dll` et `THIRD_PARTY_NOTICES.md` installés.
5. Empreinte : `sha256sum <installeur>`.
6. Étiquette `v<version>` sur le commit fusionné de `main`, puis release GitHub **en brouillon** avec l'installeur, son empreinte et les notes du changelog ; publication par l'humain.
7. L'installeur n'est pas signé (SmartScreen) tant qu'aucun certificat n'est acquis.

## Environnement de build (poste de référence)

Relevé lors de la release 0.1.3 (build réussi en 6 min 38, `npm run bundle:win`). Rien de tout cela n'est défini dans l'environnement par défaut : un terminal ordinaire échoue (pas de `cl.exe`, `BINDGEN_EXTRA_CLANG_ARGS` absent).

| Élément | Valeur sur le poste de référence |
|---|---|
| Visual Studio 2026 | `E:\Visual Studio` (pas `C:\Program Files\Microsoft Visual Studio\18`, dossier vide ; `vswhere` ne le voit pas) |
| MSVC | 14.52.37025 : `E:\Visual Studio\VC\Tools\MSVC\14.52.37025` (la version change à chaque mise à jour : `dir "E:\Visual Studio\VC\Tools\MSVC"`) |
| Windows SDK | 10.0.26100.0 : `C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0` |
| CUDA | 13.5, `CUDA_PATH` et `CUDA_PATH_V13_5` déjà définies (`C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.5`) |
| `LIBCLANG_PATH` | `D:\bin`, déjà défini |
| Rust | `CARGO_HOME=E:\Rust\cargo`, `RUSTUP_HOME=E:\Rust\rustup`, hôte `x86_64-pc-windows-msvc` |

`cl.exe` n'est pas dans le `PATH` : il faut charger l'environnement Visual Studio avec `VsDevCmd.bat`. L'avertissement « `vswhere.exe` is not recognized » qu'il affiche est sans conséquence.

Fichier `.cmd` utilisé pour la 0.1.3 (à adapter si les versions changent), à lancer depuis la racine du dépôt :

```bat
@echo off
call "E:\Visual Studio\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 exit /b 1
set "CMAKE_CUDA_ARCHITECTURES=75;86;89"
set "BINDGEN_EXTRA_CLANG_ARGS=-I"E:\Visual Studio\VC\Tools\MSVC\14.52.37025\include" -I"C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\ucrt""
node tools\copy-cuda-dlls.mjs
if errorlevel 1 exit /b 1
call npm run bundle:win
```

Contrôle rapide avant de lancer le build : `where cl.exe` doit répondre, et `echo %CMAKE_CUDA_ARCHITECTURES%` doit afficher `75;86;89`.

## Manifeste des modèles signé (N-098)

`manifests/models.json` décrit les modèles (version, fichiers, URL, taille, SHA-256) ; Navi n'utilise que ce manifeste s'il porte une signature ed25519 valide (`manifests/models.json.sig`, hexadécimal).

- Après toute modification du manifeste : `node tools/sign-manifest.mjs sign "<clé privée>" manifests/models.json`, puis commiter `models.json` et `models.json.sig` **ensemble**. Le test `manifeste_du_depot_signe_et_coherent_avec_les_modeles_epingles` échoue si la signature ne correspond plus.
- La clé privée est hors dépôt (`%USERPROFILE%\.navi-signing\navi-models-ed25519.pem`), jamais partagée, sauvegardée à deux endroits. Perdue ou compromise : `node tools/sign-manifest.mjs keygen <dossier>`, remplacer `PUBLIC_KEY` dans `src-tauri/src/models_update.rs` et publier une nouvelle version de Navi (les anciennes ne pourront plus mettre à jour leurs modèles).
- Clé publique actuelle (base64) : `wF05Qdp6E4KAZyVHR0TZiU3GI2LVq1u01erHCMfVDD0=`.
- Un modèle mis à jour garde les mêmes noms de fichiers et reste hébergé chez les mêmes fournisseurs (préfixes d'URL dans `models_store::allowed_prefix`) ; un changement de format de modèle demande d'abord une nouvelle version de Navi (`SUPPORTED_FORMAT`).
