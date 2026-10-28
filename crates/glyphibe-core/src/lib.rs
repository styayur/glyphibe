// SPDX-License-Identifier: MPL-2.0
//! A single evidence-bearing graph shared by all Glyphibe interfaces.
mod query;
mod store;
pub use query::*;
pub use store::*;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plane {
    Architecture,
    Explanation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Domain,
    Cluster,
    Component,
    File,
    Class,
    Function,
    Block,
    Statement,
    Expression,
    Concept,
    Explanation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Observed,
    Inferred,
    Proposed,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    #[default]
    Unchanged,
    Added,
    Modified,
    Removed,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceSpan {
    pub path: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl SourceSpan {
    pub fn validate(&self) -> Result<()> {
        let path = std::path::Path::new(&self.path);
        if self.path.is_empty()
            || self.path.contains('\\')
            || self.path.contains(':')
            || path.is_absolute()
            || path
                .components()
                .any(|p| !matches!(p, std::path::Component::Normal(_)))
        {
            bail!("source path must be repository-relative");
        }
        if self.start_line == 0
            || self.start_column == 0
            || self.end_line == 0
            || self.end_column == 0
            || self.end_line < self.start_line
            || self.end_byte < self.start_byte
        {
            bail!("invalid source range");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub method: String,
    pub sources: Vec<SourceSpan>,
    pub note: String,
}

impl Evidence {
    pub fn observed(method: impl Into<String>, sources: Vec<SourceSpan>) -> Self {
        Self {
            kind: EvidenceKind::Observed,
            method: method.into(),
            sources,
            note: String::new(),
        }
    }
    pub fn inferred(method: impl Into<String>, sources: Vec<SourceSpan>) -> Self {
        Self {
            kind: EvidenceKind::Inferred,
            method: method.into(),
            sources,
            note: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub level: u8,
    pub plane: Plane,
    pub repository: String,
    pub revision: String,
    pub language: Option<String>,
    pub source: Option<SourceSpan>,
    pub summary: String,
    pub evidence: Evidence,
    pub fingerprint: String,
    pub tags: Vec<String>,
    pub knowledge: Vec<String>,
    pub change: ChangeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Contains,
    Calls,
    Imports,
    References,
    DependsOn,
    Implements,
    MapsTo,
    HasConcept,
    VerifiedBy,
}

impl Relation {
    pub fn is_dependency(self) -> bool {
        matches!(
            self,
            Self::Calls | Self::Imports | Self::References | Self::DependsOn | Self::Implements
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: Relation,
    pub evidence: Evidence,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScanStats {
    pub discovered_files: usize,
    pub parsed_files: usize,
    pub cached_files: usize,
    pub skipped_files: usize,
    pub source_bytes: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub schema_version: u32,
    pub repository: String,
    pub revision: String,
    pub nodes: BTreeMap<String, Node>,
    pub edges: Vec<Edge>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: ScanStats,
    #[serde(skip)]
    edge_index: BTreeMap<String, usize>,
}

impl Graph {
    pub fn new(repository: impl Into<String>, revision: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            repository: repository.into(),
            revision: revision.into(),
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            diagnostics: Vec::new(),
            stats: ScanStats::default(),
            edge_index: BTreeMap::new(),
        }
    }
    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }
    /// Call after bulk reordering, retaining or replacing public edge storage.
    pub fn reindex_edges(&mut self) {
        for edge in &mut self.edges {
            edge.evidence.sources.sort();
            edge.evidence.sources.dedup();
        }
        self.edge_index = self
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect();
    }
    pub fn add_edge(
        &mut self,
        source: impl Into<String>,
        target: impl Into<String>,
        relation: Relation,
        mut evidence: Evidence,
    ) {
        let source = source.into();
        let target = target.into();
        let name = format!("{relation:?}");
        let id = stable_id(&[
            &source,
            &target,
            &name,
            &evidence.method,
            &format!("{:?}", evidence.kind),
        ]);
        if self.edge_index.len() != self.edges.len()
            || self
                .edge_index
                .get(&id)
                .is_some_and(|&i| self.edges.get(i).is_none_or(|edge| edge.id != id))
        {
            self.reindex_edges();
        }
        evidence.sources.sort();
        evidence.sources.dedup();
        if let Some(&index) = self.edge_index.get(&id) {
            let edge = &mut self.edges[index];
            if edge.stale {
                edge.evidence = evidence;
                edge.stale = false;
            } else {
                for span in evidence.sources {
                    if let Err(position) = edge.evidence.sources.binary_search(&span) {
                        edge.evidence.sources.insert(position, span);
                    }
                }
                if !evidence.note.is_empty() && !edge.evidence.note.contains(&evidence.note) {
                    if !edge.evidence.note.is_empty() {
                        edge.evidence.note.push('\n');
                    }
                    edge.evidence.note.push_str(&evidence.note);
                }
            }
        } else {
            self.edge_index.insert(id.clone(), self.edges.len());
            self.edges.push(Edge {
                id,
                source,
                target,
                relation,
                evidence,
                stale: false,
            });
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            bail!("unsupported graph schema");
        }
        let mut edge_ids = BTreeSet::new();
        for (key, node) in &self.nodes {
            if key != &node.id || key.is_empty() || !(1..=6).contains(&node.level) {
                bail!("invalid node identity or level");
            }
            if node.repository != self.repository {
                bail!("foreign repository node");
            }
            if let Some(source) = &node.source {
                source.validate()?;
            }
            for source in &node.evidence.sources {
                source.validate()?;
            }
        }
        for edge in &self.edges {
            if !edge_ids.insert(&edge.id) {
                bail!("duplicate edge identity");
            }
            let Some(source) = self.nodes.get(&edge.source) else {
                bail!("dangling edge source");
            };
            let Some(target) = self.nodes.get(&edge.target) else {
                bail!("dangling edge target");
            };
            if edge.relation == Relation::MapsTo && source.plane == target.plane {
                bail!("maps_to must cross planes");
            }
            if edge.relation == Relation::HasConcept && target.kind != NodeKind::Concept {
                bail!("has_concept requires a concept target");
            }
            if edge.evidence.kind == EvidenceKind::Verified
                && edge.evidence.sources.is_empty()
                && edge.evidence.note.is_empty()
            {
                bail!("verified relationship requires independent evidence");
            }
            for source in &edge.evidence.sources {
                source.validate()?;
            }
        }
        Ok(())
    }
    /// Preserve confidently matched identities; retain removed facts as stale history.
    /// Fingerprint matching is only accepted when unique on both sides and same kind/language.
    pub fn reconcile(&mut self, previous: &Graph) -> Result<()> {
        if self.repository != previous.repository {
            bail!("cannot reconcile different repositories");
        }
        let mut remap = BTreeMap::new();
        let current_ids: BTreeSet<_> = self.nodes.keys().cloned().collect();
        let mut old_fingerprints: BTreeMap<(NodeKind, Option<String>, String), Vec<String>> =
            BTreeMap::new();
        let mut new_fingerprints: BTreeMap<(NodeKind, Option<String>, String), Vec<String>> =
            BTreeMap::new();
        for node in previous.nodes.values().filter(|n| {
            n.change != ChangeStatus::Removed
                && !current_ids.contains(&n.id)
                && !n.fingerprint.is_empty()
        }) {
            old_fingerprints
                .entry((node.kind, node.language.clone(), node.fingerprint.clone()))
                .or_default()
                .push(node.id.clone());
        }
        for node in self
            .nodes
            .values()
            .filter(|n| !previous.nodes.contains_key(&n.id) && !n.fingerprint.is_empty())
        {
            new_fingerprints
                .entry((node.kind, node.language.clone(), node.fingerprint.clone()))
                .or_default()
                .push(node.id.clone());
        }
        for (key, new_ids) in &new_fingerprints {
            if let Some(old_ids) = old_fingerprints.get(key) {
                if old_ids.len() == 1 && new_ids.len() == 1 {
                    remap.insert(new_ids[0].clone(), old_ids[0].clone());
                } else {
                    for id in new_ids {
                        if let Some(node) = self.nodes.get_mut(id) {
                            node.change = ChangeStatus::Ambiguous;
                        }
                    }
                    self.diagnostics.push(Diagnostic { code: "identity_ambiguous".into(),
                        message: format!("{} old and {} new symbols share a fingerprint; identities were not guessed", old_ids.len(), new_ids.len()), path: None });
                }
            }
        }
        for (new, old) in &remap {
            if let Some(mut node) = self.nodes.remove(new) {
                node.id.clone_from(old);
                node.change = ChangeStatus::Modified;
                self.nodes.insert(old.clone(), node);
            }
        }
        for node in self.nodes.values_mut() {
            if node.change == ChangeStatus::Ambiguous {
                continue;
            }
            node.change = match previous.nodes.get(&node.id) {
                Some(old)
                    if old.fingerprint == node.fingerprint
                        && old.source == node.source
                        && old.label == node.label =>
                {
                    ChangeStatus::Unchanged
                }
                Some(_) => ChangeStatus::Modified,
                None => ChangeStatus::Added,
            };
        }
        for edge in &mut self.edges {
            if let Some(id) = remap.get(&edge.source) {
                edge.source.clone_from(id);
            }
            if let Some(id) = remap.get(&edge.target) {
                edge.target.clone_from(id);
            }
            edge.id = stable_id(&[
                &edge.source,
                &edge.target,
                &format!("{:?}", edge.relation),
                &edge.evidence.method,
                &format!("{:?}", edge.evidence.kind),
            ]);
        }
        for old in previous
            .nodes
            .values()
            .filter(|n| !self.nodes.contains_key(&n.id))
            .cloned()
            .collect::<Vec<_>>()
        {
            let mut node = old;
            node.change = ChangeStatus::Removed;
            self.add_node(node);
        }
        let edge_ids: BTreeSet<_> = self.edges.iter().map(|e| e.id.clone()).collect();
        for old in &previous.edges {
            if !edge_ids.contains(&old.id) {
                let mut edge = old.clone();
                edge.stale = true;
                self.edges.push(edge);
            }
        }
        self.reindex_edges();
        self.validate()
    }
}

/// Length-prefix inputs so identities cannot collide through delimiter injection.
pub fn stable_id(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
pub fn fingerprint(text: &str) -> String {
    stable_id(&[text])
}

/// Refuse every existing symlink/reparse component, including Windows junctions.
pub fn ensure_no_symlinks(path: &std::path::Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    for ancestor in absolute.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                let linked = metadata.file_type().is_symlink();
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    linked || metadata.file_attributes() & 0x400 != 0
                };
                if linked {
                    bail!("symbolic links and reparse points are excluded");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Open only an existing file beneath the selected canonical repository.
pub fn read_source(root: &std::path::Path, source: &SourceSpan, max_bytes: u64) -> Result<String> {
    source.validate()?;
    let root = root.canonicalize().context("repository unavailable")?;
    let requested = root.join(&source.path);
    ensure_no_symlinks(&requested)?;
    let mut prefix = root.clone();
    for part in std::path::Path::new(&source.path).components() {
        prefix.push(part);
        if std::fs::symlink_metadata(&prefix)?.file_type().is_symlink() {
            bail!("symbolic links are excluded");
        }
    }
    let resolved = requested.canonicalize()?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
        bail!("source path escapes repository");
    }
    if std::fs::metadata(&resolved)?.len() > max_bytes {
        bail!("source exceeds configured byte limit");
    }
    read_bounded_utf8(std::fs::File::open(resolved)?, max_bytes)
}

fn read_bounded_utf8(reader: impl std::io::Read, max_bytes: u64) -> Result<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    reader
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        bail!("source grew beyond configured byte limit");
    }
    String::from_utf8(bytes).context("source must be UTF-8")
}

#[cfg(test)]
mod bounded_reader_tests {
    #[test]
    fn actual_read_is_bounded_even_when_source_exceeds_prior_metadata() {
        let mut source = std::io::Cursor::new(vec![b'x'; 100_000]);
        assert!(super::read_bounded_utf8(&mut source, 19).is_err());
        assert_eq!(source.position(), 20);
        assert_eq!(
            super::read_bounded_utf8(std::io::Cursor::new(b"abc"), 3).unwrap(),
            "abc"
        );
        assert!(super::read_bounded_utf8(std::io::Cursor::new([0xff]), 3).is_err());
    }
}
