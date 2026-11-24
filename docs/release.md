# Procédure de release

Pas de CI de publication : la release est construite localement puis publiée à la main.

1. Versions alignées dans `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` ; entrée dans `CHANGELOG.md`.
2. Environnement : CUDA 13.5, Visual Studio (voir `docs/environnement-reference.md`), `CMAKE_CUDA_ARCHITECTURES="75;86;89"` (ne pas laisser `86` seul).
3. `node tools/copy-cuda-dlls.mjs` puis `npm run bundle:win` → `src-tauri/target/release/bundle/nsis/Navi_<version>_x64-setup.exe`.
4. Contrôler l'exécutable : manifeste `requireAdministrator`, DLL `cublas64_13.dll`/`cublasLt64_13.dll` et `THIRD_PARTY_NOTICES.md` installés.
5. Empreinte : `sha256sum <installeur>`.
6. Étiquette `v<version>` sur le commit fusionné de `main`, puis release GitHub **en brouillon** avec l'installeur, son empreinte et les notes du changelog ; publication par l'humain.
7. L'installeur n'est pas signé (SmartScreen) tant qu'aucun certificat n'est acquis.

## Manifeste des modèles signé (N-098)

`manifests/models.json` décrit les modèles (version, fichiers, URL, taille, SHA-256) ; Navi n'utilise que ce manifeste s'il porte une signature ed25519 valide (`manifests/models.json.sig`, hexadécimal).

- Après toute modification du manifeste : `node tools/sign-manifest.mjs sign "<clé privée>" manifests/models.json`, puis commiter `models.json` et `models.json.sig` **ensemble**. Le test `manifeste_du_depot_signe_et_coherent_avec_les_modeles_epingles` échoue si la signature ne correspond plus.
- La clé privée est hors dépôt (`%USERPROFILE%\.navi-signing\navi-models-ed25519.pem`), jamais partagée, sauvegardée à deux endroits. Perdue ou compromise : `node tools/sign-manifest.mjs keygen <dossier>`, remplacer `PUBLIC_KEY` dans `src-tauri/src/models_update.rs` et publier une nouvelle version de Navi (les anciennes ne pourront plus mettre à jour leurs modèles).
- Clé publique actuelle (base64) : `wF05Qdp6E4KAZyVHR0TZiU3GI2LVq1u01erHCMfVDD0=`.
- Un modèle mis à jour garde les mêmes noms de fichiers et reste hébergé chez les mêmes fournisseurs (préfixes d'URL dans `models_store::allowed_prefix`) ; un changement de format de modèle demande d'abord une nouvelle version de Navi (`SUPPORTED_FORMAT`).
