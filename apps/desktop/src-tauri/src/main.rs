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
    loaded: Arc<Mutex<Option<LoadedGraph>>>,
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}
struct LoadedGraph {
    root: PathBuf,
    graph: Graph,
}
struct ScanGuard {
    session: Session,
}
impl Drop for ScanGuard {
    fn drop(&mut self) {
        self.session.cancel.store(true, Ordering::SeqCst);
        self.session.running.store(false, Ordering::SeqCst);
    }
}
impl Session {
    fn begin_scan(&self) -> Result<ScanGuard, String> {
        // Serialize the running transition against reads of the loaded pair.
        let _loaded = self.loaded.lock().map_err(|e| e.to_string())?;
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("Analysis in progress; wait for completion or cancel it.".into());
        }
        self.cancel.store(false, Ordering::SeqCst);
        Ok(ScanGuard {
            session: self.clone(),
        })
    }
    fn replace(&self, root: PathBuf, graph: Graph) -> Result<(), String> {
        *self.loaded.lock().map_err(|e| e.to_string())? = Some(LoadedGraph { root, graph });
        Ok(())
    }
    fn query<T>(&self, query: impl FnOnce(&LoadedGraph) -> Result<T, String>) -> Result<T, String> {
        let loaded = self.loaded.lock().map_err(|e| e.to_string())?;
        if self.running.load(Ordering::SeqCst) {
            return Err("Analysis in progress; wait for completion or cancel it.".into());
        }
        query(loaded.as_ref().ok_or("Open a repository first.")?)
    }
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
    let _scan = state.begin_scan()?;
    let session = state.inner().clone();
    scan(app, session.clone()).await
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
    if session.cancel.load(Ordering::SeqCst) {
        return Err("Analysis cancelled.".into());
    }
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
    session.replace(root, graph.clone())?;
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
    let _scan = state.begin_scan()?;
    let session = state.inner().clone();
    scan_root(app, session.clone(), PathBuf::from(root))
        .await
        .map(Some)
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
    state.query(|loaded| {
        loaded
            .graph
            .impact(&id, direction, hops)
            .map_err(|e| e.to_string())
    })
}
#[tauri::command]
fn explain_node(
    id: String,
    depth: u8,
    language: String,
    state: tauri::State<'_, Session>,
) -> Result<serde_json::Value, String> {
    state.query(|loaded| {
        let explanation = glyphibe_explain::explain(&loaded.graph, &id, depth, &language)
            .map_err(|e| e.to_string())?;
        serde_json::to_value(explanation).map_err(|e| e.to_string())
    })
}
#[tauri::command]
fn read_source(id: String, state: tauri::State<'_, Session>) -> Result<String, String> {
    state.query(|loaded| read_node_source(&loaded.root, &loaded.graph, &id))
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
    fn scan_gate_rejects_queries_and_duplicates_and_recovers_on_drop() {
        let session = Session::default();
        session
            .replace(PathBuf::from("a"), Graph::new("a", "working"))
            .unwrap();
        let scan = session.begin_scan().unwrap();
        assert!(session.begin_scan().is_err());
        let called = AtomicBool::new(false);
        let result = session.query(|_| {
            called.store(true, Ordering::SeqCst);
            Ok(())
        });
        assert!(result.unwrap_err().contains("Analysis in progress"));
        assert!(!called.load(Ordering::SeqCst));
        session
            .replace(PathBuf::from("b"), Graph::new("b", "working"))
            .unwrap();
        assert!(session.query(|_| Ok(())).is_err());
        drop(scan);
        assert_eq!(
            session
                .query(|loaded| Ok((loaded.root.clone(), loaded.graph.repository.clone())))
                .unwrap(),
            (PathBuf::from("b"), "b".into())
        );
        assert!(session.begin_scan().is_ok());
    }
    #[test]
    fn concurrent_snapshot_replacement_never_pairs_a_root_with_another_graph() {
        let session = Session::default();
        session
            .replace(PathBuf::from("a"), Graph::new("a", "working"))
            .unwrap();
        let writer = session.clone();
        let thread = std::thread::spawn(move || {
            for index in 0..1000 {
                let name = if index % 2 == 0 { "a" } else { "b" };
                writer
                    .replace(PathBuf::from(name), Graph::new(name, "working"))
                    .unwrap();
            }
        });
        for _ in 0..1000 {
            session
                .query(|loaded| {
                    assert_eq!(loaded.root.to_string_lossy(), loaded.graph.repository);
                    Ok(())
                })
                .unwrap();
        }
        thread.join().unwrap();
    }
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
