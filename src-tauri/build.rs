fn main() {
    // N-095 : Navi s'exécute toujours en administrateur (pour saisir dans les fenêtres élevées).
    // shortcut: manifeste élevé en release seulement ; `tauri dev` / `cargo run` restent non élevés
    // (sinon CreateProcess échoue avec l'erreur 740 hors terminal administrateur). Les builds de
    // livraison (`tauri build`) sont en release.
    let mut attributes = tauri_build::Attributes::new();
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        attributes = attributes.windows_attributes(
            tauri_build::WindowsAttributes::new().app_manifest(include_str!("navi-admin.manifest")),
        );
    }
    println!("cargo:rerun-if-changed=navi-admin.manifest");
    tauri_build::try_build(attributes).expect("échec de tauri-build");
}
