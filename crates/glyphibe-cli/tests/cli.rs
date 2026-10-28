// SPDX-License-Identifier: MPL-2.0
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
};
fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn run(repo: &Path, index: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_glyphibe"))
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("GEMINI_API_KEY")
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .arg("--repo")
        .arg(repo)
        .arg("--index")
        .arg(index)
        .args(args)
        .output()
        .unwrap()
}
fn json_output(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn setup() -> (tempfile::TempDir, tempfile::TempDir, std::path::PathBuf) {
    let repo = tempdir();
    let cache = tempdir();
    std::fs::write(
        repo.path().join("maths.py"),
        "def twice(value):\n    return value * 2\n",
    )
    .unwrap();
    std::fs::write(repo.path().join("main.py"),"from maths import twice\ndef run(value):\n    if value > 0:\n        return twice(value)\n    return 0\n").unwrap();
    let index = cache.path().join("index.sqlite");
    json_output(run(repo.path(), &index, &["analyze"]));
    (repo, cache, index)
}
#[test]
fn cli_real_scan_queries_export_and_error_contract() {
    let (repo, cache, index) = setup();
    let search = json_output(run(repo.path(), &index, &["search", "run"]));
    let id = search
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "function" && n["plane"] == "architecture")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let trace = json_output(run(repo.path(), &index, &["trace", id]));
    assert!(!trace["nodes"].as_array().unwrap().is_empty());
    let explanation = json_output(run(
        repo.path(),
        &index,
        &["explain", id, "--depth", "1", "--language", "zh"],
    ));
    assert_eq!(explanation["depth"], 1);
    assert!(!explanation["steps"].as_array().unwrap().is_empty());
    let impact = json_output(run(
        repo.path(),
        &index,
        &["impact", id, "--direction", "downstream"],
    ));
    assert!(impact["functions"].as_u64().unwrap() >= 1);
    let mapping = json_output(run(repo.path(), &index, &["map", id]));
    assert!(!mapping["nodes"].as_array().unwrap().is_empty());
    let export = cache.path().join("graph.json");
    assert!(
        run(
            repo.path(),
            &index,
            &["export", "--output", export.to_str().unwrap()]
        )
        .status
        .success()
    );
    let graph: Value = serde_json::from_slice(&std::fs::read(&export).unwrap()).unwrap();
    assert_eq!(graph["schema_version"], 1);
    assert!(
        !run(
            repo.path(),
            &index,
            &["export", "--output", export.to_str().unwrap()]
        )
        .status
        .success()
    );
    assert!(
        run(
            repo.path(),
            &index,
            &["export", "--output", export.to_str().unwrap(), "--force"]
        )
        .status
        .success()
    );
    let error = run(repo.path(), &index, &["trace", "unknown"]);
    assert_eq!(error.status.code(), Some(1));
    assert!(error.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&error.stderr).unwrap()["code"],
        "glyphibe_error"
    );
    for limit in ["0", "10001"] {
        let result = run(repo.path(), &index, &["search", "run", "--limit", limit]);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
    }
    assert_eq!(
        run(repo.path(), &index, &["--unknown-flag"]).status.code(),
        Some(2)
    );
    let doctor = json_output(run(repo.path(), &index, &["doctor"]));
    assert_eq!(doctor["llm_required"], false);
    assert!(
        run(repo.path(), &repo.path().join("bad.sqlite"), &["analyze"])
            .status
            .code()
            == Some(1)
    );
}

#[tokio::test]
async fn official_mcp_sdk_client_over_real_stdio() {
    use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
    let (repo, _cache, index) = setup();
    let mut process = tokio::process::Command::new(env!("CARGO_BIN_EXE_glyphibe"));
    process
        .arg("--repo")
        .arg(repo.path())
        .arg("--index")
        .arg(&index)
        .args(["mcp", "serve"]);
    let transport = TokioChildProcess::new(process).unwrap();
    let client = ().serve(transport).await.unwrap();
    let tools = client.list_all_tools().await.unwrap();
    for name in [
        "architecture",
        "node",
        "search",
        "trace",
        "impact",
        "mappings",
        "explain",
        "knowledge",
        "propose",
        "proposals",
    ] {
        assert!(tools.iter().any(|t| t.name == name), "missing {name}");
    }
    let call = |name: &str, args: Value| {
        CallToolRequestParams::new(name.to_owned())
            .with_arguments(args.as_object().unwrap().clone())
    };
    let architecture = client
        .call_tool(call(
            "architecture",
            json!({"level":5,"plane":"architecture","limit":100}),
        ))
        .await
        .unwrap();
    assert!(!architecture.is_error.unwrap_or(false));
    let data = architecture.structured_content.unwrap();
    assert!(!data["nodes"].as_array().unwrap().is_empty());
    let id = data["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "function" && n["label"] == "run")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let twice = data["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "function" && n["label"] == "twice")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    // An induced one-node architecture page cannot contain this cross-page call.
    for selected in [id, twice] {
        let offset = data["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|node| node["id"] == selected)
            .unwrap();
        let page = client
            .call_tool(call(
                "architecture",
                json!({"level":5,"plane":"architecture","offset":offset,"limit":1}),
            ))
            .await
            .unwrap()
            .structured_content
            .unwrap();
        assert_eq!(page["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(page["edge_scope"], "induced_node_page");
        assert!(
            !page["edges"]
                .as_array()
                .unwrap()
                .iter()
                .any(|edge| edge["source"] == id && edge["target"] == twice)
        );
    }
    // Node adjacency is independent of architecture node pages and exposes every callsite.
    let first = client
        .call_tool(call(
            "node",
            json!({"id":id,"edge_offset":0,"edge_limit":1}),
        ))
        .await
        .unwrap()
        .structured_content
        .unwrap();
    let total = first["adjacency"]["total"].as_u64().unwrap();
    assert!(total > 1);
    assert_eq!(first["adjacency"]["edges"].as_array().unwrap().len(), 1);
    assert_eq!(first["adjacency"]["offset"], 0);
    assert_eq!(first["adjacency"]["limit"], 1);
    assert_eq!(first["adjacency"]["truncated"], true);
    let mut incident = Vec::new();
    for offset in 0..total {
        let page = client
            .call_tool(call(
                "node",
                json!({"id":id,"edge_offset":offset,"edge_limit":1}),
            ))
            .await
            .unwrap()
            .structured_content
            .unwrap();
        assert!(page["adjacency"]["neighbors"].as_array().unwrap().len() <= 1);
        incident.extend(
            page["adjacency"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .cloned(),
        );
        assert_eq!(page["adjacency"]["truncated"], offset + 1 < total);
    }
    let call_edge = incident
        .iter()
        .find(|e| e["relation"] == "calls" && e["source"] == id && e["target"] == twice)
        .expect("cross-page call evidence must be retrievable");
    assert_eq!(call_edge["evidence"]["kind"], "inferred");
    assert_eq!(call_edge["evidence"]["sources"][0]["path"], "main.py");
    assert!(
        call_edge["evidence"]["sources"][0]["start_line"]
            .as_u64()
            .unwrap()
            > 0
    );
    let file_id = first["adjacency"]["neighbors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "file")
        .map(|n| n["id"].as_str().unwrap().to_owned())
        .unwrap_or_else(|| {
            incident
                .iter()
                .find(|e| {
                    e["relation"] == "contains"
                        && e["target"] == id
                        && e["evidence"]["method"] == "syntax containment"
                })
                .unwrap()["source"]
                .as_str()
                .unwrap()
                .to_owned()
        });
    let file = client
        .call_tool(call("node", json!({"id":file_id,"edge_limit":1000})))
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert!(
        file["adjacency"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["relation"] == "imports"
                && edge["evidence"]["kind"] == "observed"
                && edge["evidence"]["sources"][0]["path"] == "main.py")
    );
    for limit in [0, 1001] {
        let invalid = client
            .call_tool(call("node", json!({"id":id,"edge_limit":limit})))
            .await;
        assert!(invalid.is_err() || invalid.unwrap().is_error.unwrap_or(false));
    }
    let beyond = client
        .call_tool(call(
            "node",
            json!({"id":id,"edge_offset":total+1,"edge_limit":1}),
        ))
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert!(beyond["adjacency"]["edges"].as_array().unwrap().is_empty());
    assert_eq!(beyond["adjacency"]["truncated"], false);
    for (name, args) in [
        ("node", json!({"id":id})),
        ("explain", json!({"id":id,"depth":2,"language":"en"})),
        ("impact", json!({"id":id,"direction":"both","hops":2})),
        ("search", json!({"query":"twice"})),
        ("knowledge", json!({"concept":"function"})),
    ] {
        let result = client.call_tool(call(name, args)).await.unwrap();
        assert!(!result.is_error.unwrap_or(false), "{name}");
    }
    let denied=client.call_tool(call("propose",json!({"source":id,"target":id,"relation":"depends_on","label":"candidate","rationale":"test"}))).await;
    assert!(denied.is_err() || denied.unwrap().is_error.unwrap_or(false));
    let invalid = client
        .call_tool(call("architecture", json!({"level":9})))
        .await;
    assert!(invalid.is_err() || invalid.unwrap().is_error.unwrap_or(false));
    // Old byte offsets must never be used as source for a removed historical node.
    let file = repo.path().join("main.py");
    let original = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, "def replacement(value):\n    return value + 999\n").unwrap();
    let stale = client.call_tool(call("node", json!({"id":id}))).await;
    assert!(stale.is_err() || stale.unwrap().is_error.unwrap_or(false));
    std::fs::write(&file, &original).unwrap();
    client.cancel().await.unwrap();
    std::fs::write(&file, "def replacement(value):\n    return value + 999\n").unwrap();
    json_output(run(repo.path(), &index, &["analyze"]));
    let mut process = tokio::process::Command::new(env!("CARGO_BIN_EXE_glyphibe"));
    process
        .arg("--repo")
        .arg(repo.path())
        .arg("--index")
        .arg(&index)
        .args(["mcp", "serve"]);
    let historical = ().serve(TokioChildProcess::new(process).unwrap()).await.unwrap();
    let metadata = historical
        .call_tool(call("node", json!({"id":id})))
        .await
        .unwrap();
    let metadata = metadata.structured_content.unwrap();
    assert_eq!(metadata["node"]["change"], "removed");
    assert!(metadata["source"].is_null());
    assert_eq!(metadata["source_status"], "historical_unavailable");
    assert!(
        metadata["adjacency"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["stale"] == true
                && edge["relation"] == "calls"
                && !edge["evidence"]["sources"].as_array().unwrap().is_empty())
    );
    let trace = historical.call_tool(call("trace", json!({"id":id}))).await;
    assert!(trace.is_err() || trace.unwrap().is_error.unwrap_or(false));
    historical.cancel().await.unwrap();
}

#[test]
fn disconnected_progress_stderr_never_panics_the_scanner() {
    use std::process::Stdio;
    let (repo, _cache, index) = setup();
    let mut child = Command::new(env!("CARGO_BIN_EXE_glyphibe"))
        .arg("--repo")
        .arg(repo.path())
        .arg("--index")
        .arg(&index)
        .arg("analyze")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "analysis must survive a disconnected progress pipe: {:?}",
        output.status.code()
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["stats"]["discovered_files"], 2);
}
