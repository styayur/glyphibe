fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "open_repository",
            "startup_repository",
            "cancel_scan",
            "query_impact",
            "explain_node",
            "read_source",
            "window_action",
        ]),
    ))
    .expect("Unable to generate restricted Glyphibe application permissions");
}
