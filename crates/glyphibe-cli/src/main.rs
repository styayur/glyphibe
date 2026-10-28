// SPDX-License-Identifier: MPL-2.0
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use glyphibe_core::{Direction, Graph, IndexStore, Plane, Relation, fingerprint};
use glyphibe_parser::AnalyzeOptions;
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Parser)]
#[command(
    name = "glyphibe",
    version,
    about = "Explore code as an evidence-backed local graph. No LLM or API key."
)]
struct Cli {
    /// Repository root for query commands. Never executes its code.
    #[arg(long, global = true, default_value = ".")]
    repo: PathBuf,
    /// External SQLite index. Defaults to a per-repository user cache.
    #[arg(long, global = true)]
    index: Option<PathBuf>,
    /// Machine-readable output (JSON is also the default).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Discover, parse, resolve, cluster and map a repository without executing it.
    Analyze {
        path: Option<PathBuf>,
        #[arg(long, default_value_t = 1.0)]
        resolution: f64,
        #[arg(long, default_value_t = 2_097_152)]
        max_file_bytes: u64,
        #[arg(long, default_value_t = 20_000)]
        max_files: usize,
    },
    /// Read a bounded page of the indexed graph.
    Graph {
        #[arg(long)]
        level: Option<u8>,
        #[arg(long, value_enum)]
        plane: Option<PlaneArg>,
        #[arg(long, default_value_t = 1000)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    /// Search symbol names, tags and repository-relative source paths.
    Search {
        query: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Trace a resolved static call chain.
    Trace {
        node: String,
        #[arg(long, value_enum, default_value = "downstream")]
        direction: DirectionArg,
        #[arg(long, default_value_t = 4)]
        hops: u8,
    },
    /// Explain facts with a bilingual, progressive lesson.
    Explain {
        node: String,
        #[arg(long, default_value_t = 2)]
        depth: u8,
        #[arg(long, default_value = "en")]
        language: String,
    },
    /// Explore possible effects along dependency edges, without a risk percentage.
    Impact {
        node: String,
        #[arg(long, value_enum, default_value = "upstream")]
        direction: DirectionArg,
        #[arg(long, default_value_t = 3)]
        hops: u8,
    },
    /// Follow cross-plane mappings or structural parents/children.
    Map {
        node: String,
        #[arg(long)]
        ancestors: bool,
        #[arg(long)]
        descendants: bool,
        /// Structural traversal bound, independent of the six display levels.
        #[arg(long, default_value_t = 32)]
        hops: u8,
    },
    /// Concept registry and local-only knowledge imports.
    Kb {
        #[command(subcommand)]
        command: KbCommand,
    },
    /// Export facts to JSON or Graphviz DOT. Existing files need --force.
    Export {
        #[arg(long, value_enum, default_value = "json")]
        format: ExportFormat,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Runtime capabilities and dependency-free operating requirements.
    Doctor,
    /// MCP stdio server; the selected repository is its access boundary.
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Inspect or create isolated semantic proposals, never overwrite code facts.
    Proposals {
        #[command(subcommand)]
        command: ProposalCommand,
    },
}
#[derive(Subcommand)]
enum KbCommand {
    List,
    Show {
        concept: String,
    },
    Import {
        url: String,
        #[arg(long)]
        destination: PathBuf,
    },
    /// Search only an explicitly selected local import catalog; imported text stays untrusted.
    SearchImports {
        query: String,
        #[arg(long)]
        catalog: PathBuf,
    },
}
#[derive(Subcommand)]
enum McpCommand {
    Serve {
        #[arg(long)]
        allow_proposals: bool,
    },
}
#[derive(Subcommand)]
enum ProposalCommand {
    List,
    Add {
        source: String,
        target: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        rationale: String,
    },
}
#[derive(Clone, Copy, ValueEnum)]
enum DirectionArg {
    Upstream,
    Downstream,
    Both,
}
impl From<DirectionArg> for Direction {
    fn from(d: DirectionArg) -> Self {
        match d {
            DirectionArg::Upstream => Self::Upstream,
            DirectionArg::Downstream => Self::Downstream,
            DirectionArg::Both => Self::Both,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
enum PlaneArg {
    Architecture,
    Explanation,
}
impl From<PlaneArg> for Plane {
    fn from(p: PlaneArg) -> Self {
        match p {
            PlaneArg::Architecture => Self::Architecture,
            PlaneArg::Explanation => Self::Explanation,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
enum ExportFormat {
    Json,
    Dot,
}

fn default_index(root: &Path) -> Result<PathBuf> {
    let root = root
        .canonicalize()
        .context("repository root does not exist")?;
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".cache")))
        .context("no user cache directory; set --index to an external path")?;
    Ok(base
        .join("Glyphibe")
        .join(fingerprint(&root.to_string_lossy()))
        .join("index.sqlite"))
}
fn index_path(root: &Path, explicit: Option<&Path>) -> Result<PathBuf> {
    let path = explicit
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| default_index(root))?;
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    let canonical = root.canonicalize()?;
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().context("invalid index location")?;
    }
    if ancestor.canonicalize()?.starts_with(&canonical) {
        bail!("index must be outside analyzed repository");
    }
    Ok(absolute)
}
fn load(root: &Path, index: &Path) -> Result<Graph> {
    if !index.is_file() {
        bail!("no index: run glyphibe analyze first");
    }
    let store = IndexStore::open(index)?;
    store.bind_repository(root)?;
    store
        .load_graph()?
        .context("empty index: run glyphibe analyze first")
}
fn print_json(value: Value) -> Result<()> {
    writeln!(
        std::io::stdout().lock(),
        "{}",
        serde_json::to_string_pretty(&value)?
    )?;
    Ok(())
}
fn write_export(path: Option<&Path>, text: &str, force: bool) -> Result<()> {
    if let Some(path) = path {
        if path.exists() && std::fs::symlink_metadata(path)?.file_type().is_symlink() {
            bail!("export destination cannot be a symbolic link");
        }
        let mut options = OpenOptions::new();
        options.write(true);
        if force {
            options.create(true).truncate(true);
        } else {
            options.create_new(true);
        }
        let mut file = options
            .open(path)
            .context("cannot create export; use a new path or explicit --force")?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    } else {
        std::io::stdout().lock().write_all(text.as_bytes())?;
    }
    Ok(())
}
fn dot(graph: &Graph) -> String {
    let quote = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"?\"".into());
    let mut text = String::from("digraph Glyphibe {\n");
    for n in graph.nodes.values() {
        text.push_str(&format!(
            "  {} [label={}];\n",
            quote(&n.id),
            quote(&n.label)
        ));
    }
    for e in graph.edges.iter().filter(|e| !e.stale) {
        text.push_str(&format!(
            "  {} -> {} [label={}];\n",
            quote(&e.source),
            quote(&e.target),
            quote(&format!("{:?}", e.relation))
        ));
    }
    text.push_str("}\n");
    text
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli).await {
        let _ = writeln!(
            std::io::stderr().lock(),
            "{}",
            json!({"error":error.to_string(),"code":"glyphibe_error"})
        );
        std::process::exit(1);
    }
}
async fn run(cli: Cli) -> Result<()> {
    let selected_root = match &cli.command {
        Command::Analyze {
            path: Some(path), ..
        } => path.clone(),
        _ => cli.repo.clone(),
    };
    let root = selected_root.as_path();
    // Registry and doctor do not depend on an existing project or GUI.
    match &cli.command {
        Command::Doctor => {
            return print_json(
                json!({"name":"glyphibe","version":env!("CARGO_PKG_VERSION"),
            "schema_version":glyphibe_core::SCHEMA_VERSION,"core":"local deterministic","llm_required":false,
            "api_key_required":false,"gui_required":false,"languages":["python","rust","javascript","typescript","tsx"],
            "dynamic_calls":"unresolved unless statically proven","knowledge_license":"MPL-2.0 original entries"}),
            );
        }
        Command::Kb { command } => {
            return match command {
                KbCommand::List => {
                    print_json(serde_json::to_value(glyphibe_knowledge::registry())?)
                }
                KbCommand::Show { concept } => {
                    let registry = glyphibe_knowledge::registry();
                    let entry = registry.get(concept).context("unknown concept")?;
                    print_json(
                        json!({"concept":entry,"learning_path":registry.learning_path(concept)?}),
                    )
                }
                KbCommand::Import { url, destination } => {
                    let url = url.clone();
                    let destination = destination.clone();
                    let report = tokio::task::spawn_blocking(move || {
                        glyphibe_knowledge::import_github(&url, &destination)
                    })
                    .await??;
                    print_json(serde_json::to_value(report)?)
                }
                KbCommand::SearchImports { query, catalog } => print_json(serde_json::to_value(
                    glyphibe_knowledge::search_imports(catalog, query)?,
                )?),
            };
        }
        _ => {}
    }
    let index = index_path(root, cli.index.as_deref())?;
    match cli.command {
        Command::Analyze {
            resolution,
            max_file_bytes,
            max_files,
            ..
        } => {
            if !resolution.is_finite() || resolution <= 0.0 || resolution > 10.0 {
                bail!("resolution must be >0 and <=10");
            }
            let cancel = Arc::new(AtomicBool::new(false));
            let signal = cancel.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    signal.store(true, Ordering::Relaxed);
                }
            });
            let options = AnalyzeOptions {
                cache: Some(index.clone()),
                max_file_bytes,
                max_files,
                cancel: Some(cancel),
                progress: Some(Arc::new(|p| {
                    let _ = writeln!(std::io::stderr().lock(), "{}", json!({"progress":p}));
                })),
                ..Default::default()
            };
            let mut graph = glyphibe_parser::analyze(root, &options)?;
            glyphibe_analysis::enrich(&mut graph, resolution);
            glyphibe_explain::map_explanations(&mut graph);
            graph.validate()?;
            IndexStore::open(&index)?.save_graph(&graph)?;
            print_json(
                json!({"repository":graph.repository,"revision":graph.revision,"nodes":graph.nodes.len(),
                "edges":graph.edges.len(),"stats":graph.stats,"diagnostics":graph.diagnostics}),
            )
        }
        Command::Mcp {
            command: McpCommand::Serve { allow_proposals },
        } => {
            let graph = load(root, &index)?;
            glyphibe_mcp::serve(graph, root.canonicalize()?, index, allow_proposals).await
        }
        command => {
            let graph = load(root, &index)?;
            match command {
                Command::Graph {
                    level,
                    plane,
                    limit,
                    offset,
                } => {
                    if level.is_some_and(|l| !(1..=6).contains(&l)) || limit == 0 || limit > 100_000
                    {
                        bail!("invalid level or limit");
                    }
                    let nodes: Vec<_> = graph
                        .nodes
                        .values()
                        .filter(|n| {
                            level.is_none_or(|l| n.level == l)
                                && plane.is_none_or(|p| n.plane == p.into())
                        })
                        .skip(offset)
                        .take(limit)
                        .collect();
                    let ids: std::collections::BTreeSet<_> =
                        nodes.iter().map(|n| n.id.as_str()).collect();
                    let edges: Vec<_> = graph
                        .edges
                        .iter()
                        .filter(|e| {
                            ids.contains(e.source.as_str()) && ids.contains(e.target.as_str())
                        })
                        .collect();
                    print_json(
                        json!({"schema_version":graph.schema_version,"repository":graph.repository,"revision":graph.revision,"nodes":nodes,"edges":edges,"total_nodes":graph.nodes.len(),"offset":offset,"limit":limit}),
                    )
                }
                Command::Search { query, limit } => {
                    if limit == 0 || limit > 10_000 || query.len() > 1000 {
                        bail!("search limit must be 1..10000 and query at most 1000 bytes");
                    }
                    print_json(serde_json::to_value(graph.search(&query, limit))?)
                }
                Command::Trace {
                    node,
                    direction,
                    hops,
                } => print_json(serde_json::to_value(graph.reachable(
                    &node,
                    direction.into(),
                    hops,
                    &[Relation::Calls],
                    50_000,
                )?)?),
                Command::Explain {
                    node,
                    depth,
                    language,
                } => print_json(serde_json::to_value(glyphibe_explain::explain(
                    &graph, &node, depth, &language,
                )?)?),
                Command::Impact {
                    node,
                    direction,
                    hops,
                } => print_json(serde_json::to_value(graph.impact(
                    &node,
                    direction.into(),
                    hops,
                )?)?),
                Command::Map {
                    node,
                    ancestors,
                    descendants,
                    hops,
                } => {
                    if ancestors && descendants {
                        bail!("select only one hierarchy direction");
                    }
                    print_json(serde_json::to_value(if ancestors || descendants {
                        graph.hierarchy(&node, ancestors, hops)?
                    } else {
                        graph.mappings(&node)?
                    })?)
                }
                Command::Export {
                    format,
                    output,
                    force,
                } => write_export(
                    output.as_deref(),
                    &match format {
                        ExportFormat::Json => serde_json::to_string_pretty(&graph)?,
                        ExportFormat::Dot => dot(&graph),
                    },
                    force,
                ),
                Command::Proposals { command } => {
                    let store = IndexStore::open(&index)?;
                    print_json(match command {
                        ProposalCommand::List => serde_json::to_value(store.proposals()?)?,
                        ProposalCommand::Add {
                            source,
                            target,
                            label,
                            rationale,
                        } => serde_json::to_value(store.propose(
                            &graph,
                            &source,
                            &target,
                            Relation::DependsOn,
                            &label,
                            &rationale,
                        )?)?,
                    })
                }
                _ => unreachable!("commands handled before loading graph"),
            }
        }
    }
}
