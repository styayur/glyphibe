#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use glyphibe_core::{
    ChangeStatus, Direction, Graph, Impact, IndexStore, NodeKind, fingerprint, stable_id,
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Default)]
struct Session {
    graph: Arc<Mutex<Option<Graph>>>,
    root: Arc<Mutex<Option<PathBuf>>>,
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}
#[derive(Clone, Serialize)]
struct Progress {
    phase: String,
    completed: usize,
    total: usize,
}

#[tauri::command]
async fn open_repository(
    app: tauri::AppHandle,
    state: tauri::State<'_, Session>,
) -> Result<Option<Graph>, String> {
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("An analysis is already running.".into());
    }
    let session = state.inner().clone();
    let result = scan(app, session.clone()).await;
    session.running.store(false, Ordering::SeqCst);
    result
}
async fn scan(app: tauri::AppHandle, session: Session) -> Result<Option<Graph>, String> {
    let chooser = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        chooser
            .dialog()
            .file()
            .set_title("Open repository for local analysis")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let root = picked.into_path().map_err(|e| e.to_string())?;
    scan_root(app, session, root).await.map(Some)
}
async fn scan_root(
    app: tauri::AppHandle,
    session: Session,
    root: PathBuf,
) -> Result<Graph, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if !root.is_dir() {
        return Err("Repository path must be a directory.".into());
    }
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let cache = cache_dir.join(format!("{}.sqlite", stable_id(&[&root.to_string_lossy()])));
    session.cancel.store(false, Ordering::SeqCst);
    let cancellation = session.cancel.clone();
    let root_scan = root.clone();
    let progress_app = app.clone();
    let _ = app.emit(
        "scan-progress",
        Progress {
            phase: "Discovering and parsing".into(),
            completed: 0,
            total: 0,
        },
    );
    let graph = tauri::async_runtime::spawn_blocking(move || -> Result<Graph, String> {
        let options = glyphibe_parser::AnalyzeOptions {
            cache: Some(cache.clone()),
            cancel: Some(cancellation.clone()),
            progress: Some(Arc::new(move |progress| {
                let _ = progress_app.emit("scan-progress", progress);
            })),
            ..Default::default()
        };
        let mut graph =
            glyphibe_parser::analyze(&root_scan, &options).map_err(|e| e.to_string())?;
        if cancellation.load(Ordering::SeqCst) {
            return Err("Analysis cancelled.".into());
        }
        glyphibe_analysis::enrich(&mut graph, 1.0);
        glyphibe_explain::map_explanations(&mut graph);
        if cancellation.load(Ordering::SeqCst) {
            return Err("Analysis cancelled.".into());
        }
        let store = IndexStore::open(&cache).map_err(|e| e.to_string())?;
        store
            .bind_repository(&root_scan)
            .map_err(|e| e.to_string())?;
        store.save_graph(&graph).map_err(|e| e.to_string())?;
        Ok(graph)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if session.cancel.load(Ordering::SeqCst) {
        return Err("Analysis cancelled.".into());
    }
    let _ = app.emit(
        "scan-progress",
        Progress {
            phase: "Clustering and mapping explanations".into(),
            completed: graph.stats.parsed_files,
            total: graph.stats.discovered_files,
        },
    );
    graph.validate().map_err(|e| e.to_string())?;
    *session.root.lock().map_err(|e| e.to_string())? = Some(root);
    *session.graph.lock().map_err(|e| e.to_string())? = Some(graph.clone());
    Ok(graph)
}
#[tauri::command]
async fn startup_repository(
    app: tauri::AppHandle,
    state: tauri::State<'_, Session>,
) -> Result<Option<Graph>, String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let Some(argument) = arguments.first() else {
        return Ok(None);
    };
    if argument != "--repo" {
        return Err("Usage: glyphibe-desktop [--repo PATH]".into());
    }
    let root = arguments
        .get(1)
        .ok_or("--repo requires a directory path.")?;
    if arguments.len() > 2 {
        return Err("Usage: glyphibe-desktop [--repo PATH]".into());
    }
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("An analysis is already running.".into());
    }
    let session = state.inner().clone();
    let result = scan_root(app, session.clone(), PathBuf::from(root))
        .await
        .map(Some);
    session.running.store(false, Ordering::SeqCst);
    result
}
#[tauri::command]
fn cancel_scan(state: tauri::State<'_, Session>) {
    state.cancel.store(true, Ordering::SeqCst);
}
#[tauri::command]
fn query_impact(
    id: String,
    direction: Direction,
    hops: u8,
    state: tauri::State<'_, Session>,
) -> Result<Impact, String> {
    let guard = state.graph.lock().map_err(|e| e.to_string())?;
    let graph = guard.as_ref().ok_or("Open a repository first.")?;
    graph
        .impact(&id, direction, hops)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn explain_node(
    id: String,
    depth: u8,
    language: String,
    state: tauri::State<'_, Session>,
) -> Result<serde_json::Value, String> {
    let guard = state.graph.lock().map_err(|e| e.to_string())?;
    let graph = guard.as_ref().ok_or("Open a repository first.")?;
    let explanation =
        glyphibe_explain::explain(graph, &id, depth, &language).map_err(|e| e.to_string())?;
    serde_json::to_value(explanation).map_err(|e| e.to_string())
}
#[tauri::command]
fn read_source(id: String, state: tauri::State<'_, Session>) -> Result<String, String> {
    let guard = state.graph.lock().map_err(|e| e.to_string())?;
    let graph = guard.as_ref().ok_or("Open a repository first.")?;
    let root_guard = state.root.lock().map_err(|e| e.to_string())?;
    let root = root_guard.as_ref().ok_or("Open a repository first.")?;
    read_node_source(root, graph, &id)
}
fn read_node_source(root: &Path, graph: &Graph, id: &str) -> Result<String, String> {
    let node = graph.nodes.get(id).ok_or("Unknown node.")?;
    if node.change == ChangeStatus::Removed {
        return Err("Historical source ranges cannot be applied to current files.".into());
    }
    let span = node.source.as_ref().ok_or("Node has no source span.")?;
    let source =
        glyphibe_core::read_source(root, span, 8 * 1024 * 1024).map_err(|e| e.to_string())?;
    if let Some(file) = graph.nodes.values().find(|n| {
        n.kind == NodeKind::File
            && n.change != ChangeStatus::Removed
            && n.source.as_ref().is_some_and(|s| s.path == span.path)
    }) && file.fingerprint != fingerprint(&source)
    {
        return Err("Source changed since analysis; rescan the repository.".into());
    }
    source
        .get(span.start_byte..span.end_byte)
        .map(str::to_owned)
        .ok_or("Source changed since analysis; rescan the repository.".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_ranges_are_rejected_and_removed_files_do_not_shadow_active_checks() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path().join("source");
        std::fs::create_dir(&root).unwrap();
        let path = root.join("main.py");
        std::fs::write(&path, "def previous():\n    return 1\n").unwrap();
        let options = glyphibe_parser::AnalyzeOptions {
            cache: Some(workspace.path().join("index.sqlite")),
            ..Default::default()
        };
        let old = glyphibe_parser::analyze(&root, &options).unwrap();
        let old_id = old
            .nodes
            .values()
            .find(|n| n.kind == NodeKind::Function)
            .unwrap()
            .id
            .clone();
        std::fs::write(
            &path,
            "def current():\n    print('replacement')\n    return 'new'\n",
        )
        .unwrap();
        let mut current = glyphibe_parser::analyze(&root, &options).unwrap();
        assert_eq!(current.nodes[&old_id].change, ChangeStatus::Removed);
        assert!(
            read_node_source(&root, &current, &old_id)
                .unwrap_err()
                .contains("Historical")
        );
        let active_id = current
            .nodes
            .values()
            .find(|n| n.kind == NodeKind::Function && n.change != ChangeStatus::Removed)
            .unwrap()
            .id
            .clone();
        let mut historical_file = current
            .nodes
            .values()
            .find(|n| n.kind == NodeKind::File && n.change != ChangeStatus::Removed)
            .unwrap()
            .clone();
        historical_file.id = "000-historical-file".into();
        historical_file.change = ChangeStatus::Removed;
        historical_file.fingerprint = "old".into();
        current.add_node(historical_file);
        assert!(
            read_node_source(&root, &current, &active_id)
                .unwrap()
                .contains("def current")
        );
        std::fs::write(&path, "def current():\n    return 'changed since scan'\n").unwrap();
        assert!(
            read_node_source(&root, &current, &active_id)
                .unwrap_err()
                .contains("changed since analysis")
        );
    }
}
#[tauri::command]
fn window_action(window: tauri::WebviewWindow, action: String) -> Result<(), String> {
    match action.as_str() {
        "minimize" => window.minimize(),
        "maximize" => {
            if window.is_maximized().map_err(|e| e.to_string())? {
                window.unmaximize()
            } else {
                window.maximize()
            }
        }
        "close" => window.close(),
        _ => return Err("Unknown window action.".into()),
    }
    .map_err(|e| e.to_string())
}
fn main() {
    let first = std::env::args_os().nth(1);
    if first.as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("glyphibe-desktop {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "Glyphibe local code graph desktop\n\nUsage: glyphibe-desktop [--repo PATH]\n\n--repo PATH  Open and analyze a local repository at startup\n--version    Print version\n--help       Print help"
        );
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Session::default())
        .setup(|app| {
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window_vibrancy::apply_mica(&window, Some(true));
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_repository,
            startup_repository,
            cancel_scan,
            query_impact,
            explain_node,
            read_source,
            window_action
        ])
        .run(tauri::generate_context!())
        .expect("Unable to start Glyphibe desktop");
}
