// SPDX-License-Identifier: MPL-2.0
//! Run against an existing trusted Glyphibe binary and its indexed public fixture.
use anyhow::{Context, Result, ensure};
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::json;
use std::{path::PathBuf, time::Instant};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(
        args.len() == 4,
        "usage: mcp_latency GLYPHIBE_EXE REPO INDEX OUTPUT_JSON"
    );
    let binary = PathBuf::from(&args[0]).canonicalize()?;
    let mut command = tokio::process::Command::new(&binary);
    command
        .arg("--repo")
        .arg(&args[1])
        .arg("--index")
        .arg(&args[2])
        .args(["mcp", "serve"]);
    let started = Instant::now();
    let client = ().serve(TokioChildProcess::new(command)?).await?;
    let startup_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut samples = vec![];
    for _ in 0..20 {
        let started = Instant::now();
        let result = client
            .call_tool(
                CallToolRequestParams::new("search").with_arguments(
                    json!({"query":"calculate","limit":10})
                        .as_object()
                        .context("object")?
                        .clone(),
                ),
            )
            .await?;
        ensure!(
            !result.is_error.unwrap_or(false),
            "MCP search returned an error"
        );
        ensure!(
            result
                .structured_content
                .as_ref()
                .is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty())),
            "fixture query returned no facts"
        );
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    client.cancel().await?;
    let mut ordered = samples.clone();
    ordered.sort_by(f64::total_cmp);
    let report = json!({"method":"Official rmcp client over real stdio; one initialized server; 20 successful bounded search calls; includes JSON/protocol transport, excludes model latency", "fixture":"original public Python fixture","client_version":env!("CARGO_PKG_VERSION"),"startup_ms":startup_ms,"samples_ms":samples,"median_ms":(ordered[9]+ordered[10])/2.0,"p95_ms":ordered[18],"limitation":"Small authored fixture; not a large-repository latency estimate. Server executable build profile and SHA are recorded with the benchmark report."});
    std::fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    println!("{}", report);
    Ok(())
}
