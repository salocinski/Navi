# Procédure de release

Pas de CI de publication : la release est construite localement puis publiée à la main.

1. Versions alignées dans `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` ; entrée dans `CHANGELOG.md`.
2. Environnement : CUDA 13.5, Visual Studio (voir `docs/environnement-reference.md`), `CMAKE_CUDA_ARCHITECTURES="75;86;89"` (ne pas laisser `86` seul).
3. `node tools/copy-cuda-dlls.mjs` puis `npm run bundle:win` → `src-tauri/target/release/bundle/nsis/Navi_<version>_x64-setup.exe`.
4. Contrôler l'exécutable : manifeste `requireAdministrator`, DLL `cublas64_13.dll`/`cublasLt64_13.dll` et `THIRD_PARTY_NOTICES.md` installés.
5. Empreinte : `sha256sum <installeur>`.
6. Étiquette `v<version>` sur le commit fusionné de `main`, puis release GitHub **en brouillon** avec l'installeur, son empreinte et les notes du changelog ; publication par l'humain.
7. L'installeur n'est pas signé (SmartScreen) tant qu'aucun certificat n'est acquis.
