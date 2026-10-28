// SPDX-License-Identifier: MPL-2.0
//! Deterministic bilingual teaching from graph evidence and original concepts.
use anyhow::{Context, Result, ensure};
use glyphibe_core::{
    ChangeStatus, Evidence, EvidenceKind, Graph, Node, NodeKind, Plane, Relation, SourceSpan,
    fingerprint, stable_id,
};
use glyphibe_knowledge::{Registry, registry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonStep {
    pub question: String,
    pub hint: String,
    pub answer: String,
    pub review: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub node_id: String,
    pub title: String,
    pub level: u8,
    pub depth: u8,
    pub language: String,
    pub source: Option<SourceSpan>,
    pub evidence: Evidence,
    pub facts: Vec<String>,
    pub uncertainty: Vec<String>,
    pub concepts: Vec<String>,
    pub steps: Vec<LessonStep>,
    pub static_bindings: Vec<StaticBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticBinding {
    pub node_id: String,
    pub binding_pattern: String,
    pub literal_syntax: String,
    pub source: Option<SourceSpan>,
    pub certainty: String,
}

fn static_binding(node: &Node) -> Option<StaticBinding> {
    if node.evidence.kind == EvidenceKind::Proposed {
        return None;
    }
    let binding_pattern = tag_value(node, "binding:")?;
    let value = tag_value(node, "value:")?.trim();
    if value.len() > 256 {
        return None;
    }
    let json_literal = serde_json::from_str::<serde_json::Value>(value)
        .ok()
        .is_some_and(|v| {
            matches!(
                v,
                serde_json::Value::Null
                    | serde_json::Value::Bool(_)
                    | serde_json::Value::Number(_)
                    | serde_json::Value::String(_)
            )
        });
    if !json_literal && !matches!(value, "True" | "False" | "None") {
        return None;
    }
    Some(StaticBinding {
        node_id: node.id.clone(),
        binding_pattern: binding_pattern.to_owned(),
        literal_syntax: value.to_owned(),
        source: node.source.clone(),
        certainty: "literal_rhs_if_assignment_is_reached; no_value_propagation".to_owned(),
    })
}

fn tr<'a>(zh: bool, en: &'a str, cn: &'a str) -> &'a str {
    if zh { cn } else { en }
}
fn kind_name(kind: NodeKind, zh: bool) -> &'static str {
    match kind {
        NodeKind::Domain => tr(zh, "domain", "领域"),
        NodeKind::Cluster => tr(zh, "structural cluster", "结构集群"),
        NodeKind::Component => tr(zh, "component", "组件"),
        NodeKind::File => tr(zh, "file", "文件"),
        NodeKind::Class => tr(zh, "type or class", "类型或类"),
        NodeKind::Function => tr(zh, "function", "函数"),
        NodeKind::Block => tr(zh, "code block", "代码块"),
        NodeKind::Statement => tr(zh, "statement", "语句"),
        NodeKind::Expression => tr(zh, "expression", "表达式"),
        NodeKind::Concept => tr(zh, "concept", "概念"),
        NodeKind::Explanation => tr(zh, "explanation", "解释"),
    }
}
fn concept_ids(node: &Node, knowledge: &Registry) -> Vec<String> {
    let mut ids: BTreeSet<String> = node
        .knowledge
        .iter()
        .filter(|id| knowledge.get(id).is_some())
        .cloned()
        .collect();
    for tag in &node.tags {
        if let Some(kind) = tag.strip_prefix("syntax:") {
            ids.extend(knowledge.concepts_for_syntax(kind));
        }
    }
    let kind = match node.kind {
        NodeKind::Domain => "architecture",
        NodeKind::Cluster => "dependency",
        NodeKind::Component => "component",
        NodeKind::File => "module",
        NodeKind::Class => "class",
        NodeKind::Function => "function",
        NodeKind::Block if ids.is_empty() => "scope",
        NodeKind::Statement if ids.is_empty() => "expression",
        NodeKind::Expression => "expression",
        NodeKind::Block | NodeKind::Statement | NodeKind::Concept | NodeKind::Explanation
            if !ids.is_empty() =>
        {
            return ids.into_iter().collect();
        }
        _ => "architecture",
    };
    ids.insert(kind.to_owned());
    ids.into_iter().collect()
}
fn tag_value<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.tags.iter().find_map(|t| t.strip_prefix(key))
}

/// Architecture granularity comes from the node, independently of teaching depth.
pub fn explain(graph: &Graph, id: &str, depth: u8, language: &str) -> Result<Explanation> {
    ensure!(
        (1..=4).contains(&depth),
        "explanation depth must be between 1 and 4"
    );
    ensure!(
        matches!(language, "en" | "zh" | "zh-CN"),
        "language must be en, zh or zh-CN"
    );
    let node = graph.nodes.get(id).context("unknown graph node")?;
    let zh = language.starts_with("zh");
    let knowledge = registry();
    let concepts = concept_ids(node, &knowledge);
    let active_edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| !e.stale && (e.source == id || e.target == id))
        .collect();
    let children: Vec<_> = active_edges
        .iter()
        .filter(|e| {
            e.source == id
                && e.relation == Relation::Contains
                && e.evidence.kind != EvidenceKind::Proposed
        })
        .filter_map(|e| graph.nodes.get(&e.target))
        .collect();
    let mut static_bindings: Vec<_> = std::iter::once(node)
        .chain(children.iter().copied())
        .filter_map(static_binding)
        .collect();
    static_bindings.sort_by_key(|binding| {
        binding
            .source
            .as_ref()
            .map(|s| (s.path.clone(), s.start_byte))
    });
    let outgoing: Vec<_> = active_edges
        .iter()
        .filter(|e| {
            e.source == id
                && e.relation == Relation::Calls
                && e.evidence.kind != EvidenceKind::Proposed
        })
        .filter_map(|e| graph.nodes.get(&e.target))
        .collect();
    let incoming = active_edges
        .iter()
        .filter(|e| {
            e.target == id
                && e.relation == Relation::Calls
                && e.evidence.kind != EvidenceKind::Proposed
        })
        .count();
    let mut facts = vec![format!(
        "{}: {} (L{}).",
        kind_name(node.kind, zh),
        node.label,
        node.level
    )];
    if let Some(source) = &node.source {
        facts.push(format!(
            "{} {}:{}:{}–{}:{}.",
            tr(zh, "Source:", "源码位置："),
            source.path,
            source.start_line,
            source.start_column,
            source.end_line,
            source.end_column
        ));
    } else {
        facts.push(
            tr(
                zh,
                "This aggregate has no single source range; inspect its evidence and member edges.",
                "此聚合节点没有唯一源码范围；可查看其证据和成员关系。",
            )
            .into(),
        );
    }
    let summary: String = node
        .summary
        .chars()
        .take(if depth == 1 { 120 } else { 360 })
        .collect();
    if !summary.is_empty() {
        facts.push(format!(
            "{} {}",
            tr(zh, "Recorded structure:", "已记录结构："),
            summary
        ));
    }
    let input = tag_value(node, "input:");
    let output = tag_value(node, "output:");
    if node.kind == NodeKind::Function {
        facts.push(match input {
            Some(value) => format!("{} {value}", tr(zh, "Declared inputs:", "声明的输入：")),
            None => tr(
                zh,
                "Input parameters have not been extracted for this node. Inspect its declaration.",
                "当前节点尚未提取输入参数，请查看声明。",
            )
            .into(),
        });
        facts.push(match output { Some(value) => format!("{} {value}", tr(zh, "Declared output syntax:", "声明的输出语法：")), None => tr(zh, "No output type was extracted. Return sites describe syntax; actual results may depend on inputs.", "尚未提取输出类型。返回位置描述语法；实际结果可能依赖输入。").into() });
    }
    for (key, en, cn) in [
        ("condition:", "Branch test:", "分支条件："),
        ("iterator:", "Loop input:", "循环输入："),
        ("assignment:", "Assignment syntax:", "赋值语法："),
    ] {
        if let Some(value) = tag_value(node, key) {
            facts.push(format!("{} {value}", tr(zh, en, cn)));
        }
    }
    if depth >= 2 {
        for binding in &static_bindings {
            facts.push(format!("{} {} = {}. {}", tr(zh, "Literal assignment:", "字面量赋值："), binding.binding_pattern, binding.literal_syntax, tr(zh, "This RHS is known if the statement is reached; later values and skipped paths are not inferred.", "若执行到该语句，右侧字面量已知；不推断后续值或被跳过的路径。")));
        }
        facts.push(format!(
            "{} {}. {} {}.",
            tr(zh, "Direct structural members:", "直接结构成员："),
            children.len(),
            tr(zh, "Resolved incoming calls:", "已解析的传入调用："),
            incoming
        ));
        facts.push(format!(
            "{} {}",
            tr(zh, "Resolved outgoing calls:", "已解析的传出调用："),
            if outgoing.is_empty() {
                tr(
                    zh,
                    "none recorded; this does not prove none exist",
                    "未记录；这不证明不存在调用",
                )
                .into()
            } else {
                outgoing
                    .iter()
                    .map(|n| n.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ));
        let paths = children
            .iter()
            .filter(|n| n.kind == NodeKind::Block)
            .count();
        facts.push(format!(
            "{} {paths}. {}",
            tr(
                zh,
                "Direct control/implementation blocks:",
                "直接控制或实现块："
            ),
            tr(
                zh,
                "Select a block to inspect its recorded condition, loop or error-handling syntax.",
                "选择代码块可查看已记录的条件、循环或错误处理语法。"
            )
        ));
    }
    if depth >= 3 {
        for concept in concepts
            .iter()
            .take(if depth == 3 { 2 } else { 6 })
            .filter_map(|id| knowledge.get(id))
        {
            facts.push(format!(
                "{}: {}",
                concept.label.text(language),
                concept.description.text(language)
            ));
        }
    }
    if depth == 4 {
        facts.push(format!(
            "{} {:?}; {} {}. {} {}.",
            tr(zh, "Node evidence:", "节点证据："),
            node.evidence.kind,
            tr(zh, "Method:", "方法："),
            node.evidence.method,
            tr(zh, "Revision:", "修订："),
            node.revision
        ));
        let inferred = active_edges
            .iter()
            .filter(|e| e.evidence.kind == EvidenceKind::Inferred)
            .count();
        let proposed = active_edges
            .iter()
            .filter(|e| e.evidence.kind == EvidenceKind::Proposed)
            .count();
        facts.push(format!(
            "{} {inferred}; {} {proposed}.",
            tr(zh, "Incident inferred relations:", "相邻推断关系："),
            tr(zh, "Incident proposals:", "相邻提案：")
        ));
    }
    let mut uncertainty = vec![tr(zh, "Business purpose is not established by syntax or connectivity. No runtime values or execution order are claimed.", "语法或连接关系不能单独证明业务目的。此解释不声称已知运行时数值或执行顺序。").to_owned()];
    if node.evidence.kind != EvidenceKind::Observed {
        uncertainty.push(format!(
            "{} {:?}.",
            tr(zh, "This node is supported by", "此节点的证据类别为"),
            node.evidence.kind
        ));
    }
    if node.change == ChangeStatus::Removed {
        uncertainty.push(
            tr(
                zh,
                "This node is historical and removed from the current scan.",
                "此节点属于历史记录，当前扫描已移除。",
            )
            .into(),
        );
    }
    if node.change == ChangeStatus::Ambiguous {
        uncertainty.push(tr(zh, "Identity reconciliation was ambiguous; do not assume continuity with an earlier symbol.", "身份匹配存在歧义，不能假设它与早期符号连续对应。").into());
    }
    for diagnostic in graph
        .diagnostics
        .iter()
        .filter(|d| d.path.is_none() || d.path.as_ref() == node.source.as_ref().map(|s| &s.path))
        .take(8)
    {
        uncertainty.push(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    if active_edges
        .iter()
        .any(|e| e.evidence.kind == EvidenceKind::Proposed)
    {
        uncertainty.push(
            tr(
                zh,
                "Agent/user proposals are separate hypotheses, not static facts.",
                "Agent 或用户提案属于独立假设，不是静态事实。",
            )
            .into(),
        );
    }
    let concept = concepts
        .iter()
        .filter_map(|id| knowledge.get(id))
        .next()
        .context("no teaching concept")?;
    let steps = vec![
        LessonStep { question: tr(zh, "What evidence tells you what this node is?", "哪些证据说明这个节点是什么？").into(), hint: tr(zh, "Look at its kind, level and source location.", "查看类型、层级和源码位置。").into(), answer: facts[0].clone(), review: tr(zh, "Separate the observed structure from a guessed business purpose.", "区分观察到的结构与猜测的业务目的。").into() },
        LessonStep { question: tr(zh, "What enters this region, and what result can leave it?", "什么进入这段代码？什么结果可能离开它？").into(), hint: tr(zh, "Inspect declared parameters, return annotations and return statements. Missing metadata means unknown.", "查看声明参数、返回标注和返回语句。缺少元数据意味着未知。").into(), answer: format!("{} {}. {} {}.", tr(zh, "Input:", "输入："), input.unwrap_or(tr(zh, "not extracted", "尚未提取")), tr(zh, "Output:", "输出："), output.unwrap_or(tr(zh, "inspect return sites; actual values unknown", "需查看返回位置；实际数值未知"))), review: tr(zh, "A type annotation and a concrete runtime value are different facts.", "类型标注与具体运行时值是不同事实。").into() },
        LessonStep { question: tr(zh, "Which relationships can you follow, and what remains uncertain?", "可以沿哪些关系继续追踪？哪些部分仍不确定？").into(), hint: tr(zh, "Follow calls and contains edges; inspect each edge's evidence kind.", "沿 calls 和 contains 关系查看，并检查每条边的证据类别。").into(), answer: format!("{} {}; {} {}. {}", tr(zh, "Resolved outgoing calls:", "已解析传出调用："), outgoing.len(), tr(zh, "Structural members:", "结构成员："), children.len(), uncertainty[0]), review: tr(zh, "Reachability describes possible dependency impact, not certain runtime failure.", "可达性表示潜在依赖影响，不表示必然运行故障。").into() },
        LessonStep { question: format!("{} {}?", tr(zh, "Why understand", "为什么需要理解"), concept.label.text(language)), hint: tr(zh, "Open the concept and its prerequisite learning route.", "打开概念及其前置学习路线。").into(), answer: concept.description.text(language).into(), review: tr(zh, "Return to the source and identify one concrete place where this concept applies. Ask an Agent only for an explicitly marked proposal when evidence is insufficient.", "回到源码，指出该概念对应的一个具体位置。证据不足时，可请求 Agent 提供明确标记的提案。").into() },
    ];
    let mut evidence = Evidence::inferred(
        "deterministic teaching template",
        node.source.clone().into_iter().collect(),
    );
    evidence.note = "Derived from graph syntax, evidence labels and original knowledge. No model invocation or business-intent inference.".into();
    Ok(Explanation {
        node_id: id.into(),
        title: format!("{} · {}", node.label, kind_name(node.kind, zh)),
        level: node.level,
        depth,
        language: language.into(),
        source: node.source.clone(),
        evidence,
        facts,
        uncertainty,
        concepts,
        steps,
        static_bindings,
    })
}

/// Build canonical explanations plus explicit member-to-scope mappings. A member
/// can map to its own explanation and several enclosing aggregate explanations;
/// an aggregate explanation can cover many members. No strict tree is assumed.
pub fn map_explanations(graph: &mut Graph) {
    let knowledge = registry();
    let architecture: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| {
            n.plane == Plane::Architecture
                && n.change != ChangeStatus::Removed
                && n.evidence.kind != EvidenceKind::Proposed
        })
        .cloned()
        .collect();
    let memberships: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| {
            e.relation == Relation::Contains
                && !e.stale
                && e.evidence.kind != EvidenceKind::Proposed
        })
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();
    let live: BTreeSet<_> = architecture.iter().map(|n| n.id.clone()).collect();
    let live_explanations: BTreeSet<_> = live
        .iter()
        .map(|id| stable_id(&["explanation", id]))
        .collect();
    // Regenerate current template edges so changed ranges replace old evidence.
    // Keep historical edges explicitly stale. Caller-authored mappings are untouched.
    graph
        .edges
        .retain_mut(|edge| match edge.evidence.method.as_str() {
            "glyphibe canonical explanation" | "glyphibe scope explanation" => {
                let historical =
                    !live.contains(&edge.source) || !live_explanations.contains(&edge.target);
                edge.stale = historical;
                historical
            }
            "syntax concept association" => {
                let historical =
                    !live.contains(&edge.source) && !live_explanations.contains(&edge.source);
                edge.stale = historical;
                historical
            }
            _ => true,
        });
    graph.reindex_edges();
    for node in &architecture {
        let id = stable_id(&["explanation", &node.id]);
        let mut evidence = Evidence::inferred(
            "glyphibe canonical explanation",
            node.source.clone().into_iter().collect(),
        );
        evidence.note =
            "Deterministic structural explanation; business meaning remains unverified.".into();
        let mut explanation = node.clone();
        explanation.id = id.clone();
        explanation.kind = NodeKind::Explanation;
        explanation.plane = Plane::Explanation;
        explanation.label = format!("Explain {}", node.label);
        explanation.summary = format!("L{} {:?}: {}", node.level, node.kind, node.summary);
        explanation.evidence = evidence.clone();
        explanation.fingerprint = fingerprint(&explanation.summary);
        explanation.knowledge = concept_ids(node, &knowledge);
        graph.add_node(explanation);
        graph.add_edge(node.id.clone(), id.clone(), Relation::MapsTo, evidence);
        for concept_id in concept_ids(node, &knowledge) {
            let Some(concept) = knowledge.get(&concept_id) else {
                continue;
            };
            let target = stable_id(&["concept", &concept_id]);
            let evidence = Evidence::inferred(
                "syntax concept association",
                node.source.clone().into_iter().collect(),
            );
            {
                graph.add_node(Node {
                    id: target.clone(),
                    label: concept.label.en.clone(),
                    kind: NodeKind::Concept,
                    level: 6,
                    plane: Plane::Explanation,
                    repository: graph.repository.clone(),
                    revision: graph.revision.clone(),
                    language: None,
                    source: None,
                    summary: concept.description.en.clone(),
                    evidence: Evidence::observed("original MPL-2.0 knowledge registry", vec![]),
                    fingerprint: fingerprint(&concept.description.en),
                    tags: vec!["original-knowledge".into()],
                    knowledge: vec![concept_id.clone()],
                    change: ChangeStatus::Unchanged,
                });
            }
            graph.add_edge(
                node.id.clone(),
                target.clone(),
                Relation::HasConcept,
                evidence.clone(),
            );
            graph.add_edge(id.clone(), target, Relation::HasConcept, evidence);
        }
    }
    for (parent, child) in memberships {
        if live.contains(&parent) && live.contains(&child) {
            let target = stable_id(&["explanation", &parent]);
            let mut evidence = Evidence::inferred(
                "glyphibe scope explanation",
                graph
                    .nodes
                    .get(&child)
                    .and_then(|n| n.source.clone())
                    .into_iter()
                    .collect(),
            );
            evidence.note = "The parent explanation covers this directly declared member. This is scope coverage, not a business-purpose claim.".into();
            graph.add_edge(child, target, Relation::MapsTo, evidence);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str, level: u8, kind: NodeKind) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            kind,
            level,
            plane: Plane::Architecture,
            repository: "test".into(),
            revision: "working".into(),
            language: Some("python".into()),
            source: Some(SourceSpan {
                path: "main.py".into(),
                start_line: 1,
                start_column: 1,
                end_line: 2,
                end_column: 1,
                start_byte: 0,
                end_byte: 20,
            }),
            summary: "def twice(value): return value + value".into(),
            evidence: Evidence::observed("tree-sitter syntax", vec![]),
            fingerprint: "body".into(),
            tags: vec!["syntax:function_definition".into(), "input:(value)".into()],
            knowledge: vec![],
            change: ChangeStatus::Added,
        }
    }
    #[test]
    fn depth_and_granularity_are_independent_and_languages_work_offline() {
        let mut graph = Graph::new("test", "working");
        graph.add_node(node("twice", 5, NodeKind::Function));
        let basic = explain(&graph, "twice", 1, "en").unwrap();
        let expert = explain(&graph, "twice", 4, "zh").unwrap();
        assert_eq!(basic.level, expert.level);
        assert!(expert.facts.len() > basic.facts.len());
        assert!(expert.facts.iter().any(|s| s.contains("声明的输入")));
        assert!(
            basic
                .uncertainty
                .iter()
                .any(|s| s.contains("Business purpose"))
        );
        assert!(expert.steps.iter().all(|s| !s.question.is_empty()
            && !s.hint.is_empty()
            && !s.answer.is_empty()
            && !s.review.is_empty()));
    }
    #[test]
    fn unknown_nodes_and_invalid_controls_are_errors() {
        let graph = Graph::new("test", "working");
        assert!(explain(&graph, "absent", 1, "en").is_err());
        assert!(explain(&graph, "absent", 0, "en").is_err());
        assert!(explain(&graph, "absent", 1, "fr").is_err());
    }
    #[test]
    fn mapping_is_many_to_many_cross_plane_and_idempotent() {
        let mut graph = Graph::new("test", "working");
        graph.add_node(node("file-a", 4, NodeKind::File));
        graph.add_node(node("file-b", 4, NodeKind::File));
        graph.add_node(node("f", 5, NodeKind::Function));
        graph.add_node(node("g", 5, NodeKind::Function));
        graph.add_edge(
            "file-a",
            "f",
            Relation::Contains,
            Evidence::observed("fixture", vec![]),
        );
        graph.add_edge(
            "file-b",
            "f",
            Relation::Contains,
            Evidence::inferred("fixture", vec![]),
        );
        graph.add_edge(
            "file-a",
            "g",
            Relation::Contains,
            Evidence::observed("fixture", vec![]),
        );
        map_explanations(&mut graph);
        graph.validate().unwrap();
        let count = graph.edges.len();
        let nodes = graph.nodes.len();
        map_explanations(&mut graph);
        assert_eq!((graph.nodes.len(), graph.edges.len()), (nodes, count));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.source == "f" && e.relation == Relation::MapsTo)
                .count(),
            3
        );
        let aggregate = stable_id(&["explanation", "file-a"]);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.target == aggregate && e.relation == Relation::MapsTo)
                .count(),
            3
        );
        assert!(
            graph
                .edges
                .iter()
                .filter(|e| e.relation == Relation::MapsTo)
                .all(|e| e.evidence.kind == EvidenceKind::Inferred)
        );
    }
    #[test]
    fn six_levels_receive_explanations_and_removed_mappings_are_stale() {
        let mut graph = Graph::new("test", "working");
        for level in 1..=6 {
            graph.add_node(node(&format!("level-{level}"), level, NodeKind::Block));
        }
        map_explanations(&mut graph);
        assert_eq!(
            graph
                .nodes
                .values()
                .filter(|n| n.kind == NodeKind::Explanation)
                .map(|n| n.level)
                .collect::<BTreeSet<_>>(),
            (1..=6).collect()
        );
        graph.nodes.get_mut("level-5").unwrap().change = ChangeStatus::Removed;
        map_explanations(&mut graph);
        assert!(
            graph
                .edges
                .iter()
                .filter(|e| e.source == "level-5" && e.relation == Relation::MapsTo)
                .all(|e| e.stale)
        );
        assert!(
            explain(&graph, "level-5", 2, "en")
                .unwrap()
                .uncertainty
                .iter()
                .any(|s| s.contains("historical"))
        );
    }

    #[test]
    fn refreshed_ranges_and_removed_scopes_replace_current_mapping_evidence() {
        let mut graph = Graph::new("test", "working");
        graph.add_node(node("parent", 4, NodeKind::File));
        graph.add_node(node("child", 5, NodeKind::Function));
        graph.add_edge(
            "parent",
            "child",
            Relation::Contains,
            Evidence::observed("fixture", vec![]),
        );
        map_explanations(&mut graph);
        graph
            .nodes
            .get_mut("child")
            .unwrap()
            .source
            .as_mut()
            .unwrap()
            .start_line = 2;
        graph.nodes.get_mut("parent").unwrap().change = ChangeStatus::Removed;
        map_explanations(&mut graph);
        let own = graph
            .edges
            .iter()
            .find(|e| e.source == "child" && e.evidence.method == "glyphibe canonical explanation")
            .unwrap();
        assert!(!own.stale);
        assert_eq!(own.evidence.sources[0].start_line, 2);
        let old_parent = stable_id(&["explanation", "parent"]);
        assert!(
            graph
                .edges
                .iter()
                .filter(|e| e.source == "child" && e.target == old_parent)
                .all(|e| e.stale)
        );
        graph.validate().unwrap();
    }

    #[test]
    fn literal_binding_sites_do_not_evaluate_expressions_or_propagate_runtime_values() {
        let mut graph = Graph::new("test", "working");
        let mut assignment = node("assignment", 6, NodeKind::Statement);
        assignment.tags = vec![
            "binding:count".into(),
            "value:42".into(),
            "syntax:assignment".into(),
        ];
        graph.add_node(assignment.clone());
        let explanation = explain(&graph, "assignment", 2, "en").unwrap();
        assert_eq!(explanation.static_bindings.len(), 1);
        assert_eq!(explanation.static_bindings[0].literal_syntax, "42");
        assert!(
            explanation.static_bindings[0]
                .certainty
                .contains("if_assignment_is_reached")
        );
        assignment.tags[1] = "value:dangerous_call()".into();
        graph.add_node(assignment);
        assert!(
            explain(&graph, "assignment", 2, "en")
                .unwrap()
                .static_bindings
                .is_empty()
        );
    }

    #[test]
    fn proposals_are_visible_as_uncertainty_and_never_promoted_to_call_facts() {
        let mut graph = Graph::new("test", "working");
        graph.add_node(node("function", 5, NodeKind::Function));
        graph.add_node(node("target", 5, NodeKind::Function));
        let mut evidence = Evidence::inferred("agent proposal", vec![]);
        evidence.kind = EvidenceKind::Proposed;
        graph.add_edge("function", "target", Relation::Calls, evidence.clone());
        graph.add_edge("function", "target", Relation::Contains, evidence);
        let explanation = explain(&graph, "function", 2, "en").unwrap();
        assert!(
            explanation
                .facts
                .iter()
                .any(|fact| fact.contains("Resolved outgoing calls: none recorded"))
        );
        assert!(
            explanation
                .uncertainty
                .iter()
                .any(|fact| fact.contains("proposals are separate hypotheses"))
        );
        map_explanations(&mut graph);
        let aggregate = stable_id(&["explanation", "function"]);
        assert!(!graph.edges.iter().any(|edge| edge.source == "target"
            && edge.target == aggregate
            && edge.relation == Relation::MapsTo));
    }
}
