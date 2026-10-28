#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Measure the real CLI on generated source and optional read-only local projects.

No dependencies are installed and no analyzed project is executed. Temporary source,
indices and UI graph exports stay in --work-dir, outside analyzed repositories.
"""
from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import platform
import statistics
import subprocess
import tempfile
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path


def system_memory_bytes() -> int | None:
    if os.name == "nt":
        from ctypes import wintypes
        class Memory(ctypes.Structure):
            _fields_ = [("length", wintypes.DWORD), ("load", wintypes.DWORD)] + [
                (key, ctypes.c_ulonglong) for key in (
                    "total_phys", "avail_phys", "total_page", "avail_page",
                    "total_virtual", "avail_virtual", "avail_extended")
            ]
        value = Memory()
        value.length = ctypes.sizeof(value)
        if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(value)):
            return int(value.total_phys)
    return None


def safe_error(error: Exception) -> str:
    return str(error) if isinstance(error, RuntimeError) else f"{type(error).__name__}; details omitted to avoid local-path disclosure"


def windows_memory(process: subprocess.Popen) -> int | None:
    if os.name != "nt":
        try:
            for line in Path(f"/proc/{process.pid}/status").read_text().splitlines():
                if line.startswith("VmHWM:"):
                    return int(line.split()[1]) * 1024
        except (OSError, ValueError):
            pass
        return None
    from ctypes import wintypes

    class Counters(ctypes.Structure):
        _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
            (name, ctypes.c_size_t)
            for name in (
                "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
                "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage",
                "PagefileUsage", "PeakPagefileUsage", "PrivateUsage",
            )
        ]

    memory = Counters()
    memory.cb = ctypes.sizeof(memory)
    fn = ctypes.windll.psapi.GetProcessMemoryInfo
    fn.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
    fn.restype = wintypes.BOOL
    if fn(wintypes.HANDLE(int(process._handle)), ctypes.byref(memory), memory.cb):
        return int(memory.PeakWorkingSetSize)
    return None


def run(binary: Path, arguments: list[str], timeout: float = 180) -> tuple[dict, object]:
    started = time.perf_counter()
    peak = None
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen([str(binary), *arguments], stdout=stdout, stderr=stderr)
        while True:
            measured = windows_memory(process)
            if measured is not None:
                peak = max(peak or 0, measured)
            if (peak or 0) > 2 * 1024**3:
                process.kill()
                process.wait()
                raise RuntimeError("CLI exceeded 2 GiB process memory safety budget")
            if time.perf_counter() - started > timeout:
                process.kill()
                process.wait()
                raise RuntimeError("CLI exceeded benchmark subprocess time budget")
            try:
                process.wait(timeout=0.005)
                break
            except subprocess.TimeoutExpired:
                pass
        elapsed = (time.perf_counter() - started) * 1000
        stdout.seek(0)
        data = stdout.read(128 * 1024**2 + 1)
        if len(data) > 128 * 1024**2:
            raise RuntimeError("CLI output exceeded benchmark limit")
        if process.returncode:
            # Do not publish untrusted diagnostics, source text or local paths.
            raise RuntimeError(f"CLI exited with code {process.returncode}")
        return {"elapsed_ms": round(elapsed, 3), "peak_working_set_bytes": peak}, json.loads(data)


def cli(binary: Path, source: Path, index: Path, *arguments: str):
    return run(binary, ["--repo", str(source), "--index", str(index), *arguments])


def workload(source: Path, count: int) -> int:
    source.mkdir()
    for start in range(0, count, 100):
        stop = min(start + 100, count)
        lines = ["# SPDX-License-Identifier: MPL-2.0\n# Generated benchmark source; never executed.\n"]
        if start:
            lines.append(f"from shard_{(start-1)//100:04d} import f_{start-1}\n")
        for i in range(start, stop):
            expression = "value" if i == 0 else f"f_{i-1}(value) + 1"
            lines.append(f"def f_{i}(value):\n    return {expression}\n\n")
        (source / f"shard_{start//100:04d}.py").write_text("".join(lines), encoding="utf-8")
    return sum(path.stat().st_size for path in source.glob("*.py"))


def query_latency(binary: Path, source: Path, index: Path) -> dict:
    measurements = [cli(binary, source, index, "search", "f_", "--limit", "10")[0]["elapsed_ms"] for _ in range(5)]
    return {"samples_ms": measurements, "median_ms": statistics.median(measurements), "includes_process_start_and_graph_load": True}


def synthetic(binary: Path, parent: Path, count: int) -> dict:
    case = Path(tempfile.mkdtemp(prefix=f"source-{count}-", dir=parent))
    source = case / "source"
    index = case / "index.sqlite"
    source_bytes = workload(source, count)
    cold, cold_data = cli(binary, source, index, "analyze")
    hot_runs = [cli(binary, source, index, "analyze") for _ in range(5)]
    node_counts = [cold_data["nodes"], *[data["nodes"] for _, data in hot_runs]]
    source_file = source / "shard_0000.py"
    # A real literal edit forces a single-file parse and relation rebuild.
    source_file.write_text(source_file.read_text(encoding="utf-8").replace("return value\n", "return value + 0\n", 1), encoding="utf-8")
    incremental, incremental_data = cli(binary, source, index, "analyze")
    exported, page = cli(binary, source, index, "graph", "--plane", "architecture", "--level", "5", "--limit", "100000")
    nodes = {node["id"]: node for node in page["nodes"] if node["kind"] == "function" and node["change"] != "removed"}
    edges = [edge for edge in page["edges"] if edge["relation"] == "calls" and not edge["stale"] and edge["source"] in nodes and edge["target"] in nodes]
    graph = {"schema_version": page["schema_version"], "repository": page["repository"], "revision": page["revision"], "nodes": nodes, "edges": edges, "diagnostics": [], "stats": incremental_data["stats"]}
    destination = parent / f"graph-{count}.json"
    destination.write_text(json.dumps(graph, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    expected_edges = count - 1
    if len(nodes) != count or len(edges) != expected_edges:
        raise RuntimeError(f"Generated source graph mismatch: {len(nodes)} nodes/{len(edges)} calls, expected {count}/{expected_edges}")
    return {"functions": count, "source_bytes": source_bytes, "source_files": (count+99)//100,
            "cold": {**cold, "stats": cold_data["stats"]},
            "hot": [{**metrics, "stats": data["stats"]} for metrics, data in hot_runs],
            "hot_node_counts": node_counts, "hot_graph_size_stable": len(set(node_counts)) == 1,
            "incremental": {**incremental, "stats": incremental_data["stats"]},
            "index_bytes": index.stat().st_size, "query": query_latency(binary, source, index),
            "ui_export": {"file": destination.name, "functions": len(nodes), "calls": len(edges), "bytes": destination.stat().st_size,
                          "sha256": hashlib.sha256(destination.read_bytes()).hexdigest(), "export_ms": exported["elapsed_ms"]}}


def accuracy(binary: Path, parent: Path, repo: Path) -> dict:
    result = {}
    totals = {"true_positive": 0, "false_positive": 0, "false_negative": 0}
    for language in ("python", "rust", "javascript", "typescript"):
        source = repo / "tests" / "fixtures" / language
        index = Path(tempfile.mkdtemp(prefix=f"accuracy-{language}-", dir=parent)) / "index.sqlite"
        cli(binary, source, index, "analyze")
        _, page = cli(binary, source, index, "graph", "--plane", "architecture", "--level", "5", "--limit", "100000")
        labels = {node["id"]: node["label"] for node in page["nodes"]}
        observed = {(labels[e["source"]], labels[e["target"]]) for e in page["edges"] if e["relation"] == "calls" and not e["stale"]}
        expected = {tuple(pair) for pair in json.loads((source / "calls.expected.json").read_text())}
        measures = {"true_positive": len(observed & expected), "false_positive": len(observed - expected), "false_negative": len(expected - observed)}
        result[language] = measures
        for key, value in measures.items():
            totals[key] += value
    result["total"] = {**totals, "precision": totals["true_positive"] / max(1, totals["true_positive"] + totals["false_positive"]),
                       "recall": totals["true_positive"] / max(1, totals["true_positive"] + totals["false_negative"])}
    result["limitation"] = "47 distinct caller/callee pairs manually annotated in small authored fixtures. Not an estimate of real-world precision or recall. Dynamic callback calls are deliberately unresolved."
    return result


def real_scan(binary: Path, parent: Path, source: Path, number: int) -> dict:
    index = Path(tempfile.mkdtemp(prefix=f"readonly-case-{number}-", dir=parent)) / "index.sqlite"
    measured, data = cli(binary, source, index, "analyze")
    codes = {}
    for diagnostic in data["diagnostics"]:
        codes[diagnostic["code"]] = codes.get(diagnostic["code"], 0) + 1
    return {"case": number, "mode": "read-only source analysis; project code and dependencies never executed", **measured,
            "nodes": data["nodes"], "edges": data["edges"], "stats": data["stats"], "diagnostic_counts": codes,
            "index_bytes": index.stat().st_size, "privacy": "Source names, labels, paths and diagnostic messages intentionally omitted"}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--sizes", type=int, nargs="+", default=[1000, 5000, 10000])
    parser.add_argument("--real", type=Path, nargs="*", default=[])
    args = parser.parse_args()
    args.work_dir.mkdir(parents=True, exist_ok=True)
    version = subprocess.check_output([str(args.binary), "--version"], text=True).strip()
    cpu = platform.processor()
    if os.name == "nt":
        import winreg
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0") as key:
            cpu = winreg.QueryValueEx(key, "ProcessorNameString")[0].strip()
    report = {"measured_at": datetime.now(timezone(timedelta(hours=8))).isoformat(), "binary_version": version,
              "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              "build_profile": "release; workspace opt-level default3, thin LTO, strip" if args.binary.parent.name == "release" else "unidentified; consult artifact build provenance", "hardware": {"cpu": cpu, "logical_processors": os.cpu_count(), "os": platform.platform(), "python": platform.python_version(), "system_memory_bytes": system_memory_bytes()},
              "method": {"cold": "New external SQLite index; OS filesystem cache not flushed", "hot": "Five fresh CLI processes; unchanged file cache; relationships, clusters and mappings rebuilt",
                         "incremental": "One source file literal edited, parse cache retained; whole relation/derived graph rebuilt",
                         "peak_memory": "Windows GetProcessMemoryInfo PeakWorkingSetSize polled; excludes harness and short-lived Git metadata subprocess",
                         "query": "Five CLI subprocess searches including startup, SQLite graph deserialization and query",
                         "safety": "180 seconds and 2GiB maximum per measured subprocess"}, "synthetic": [], "real": [], "failures": []}
    for count in args.sizes:
        print(f"Measuring generated source workload: {count} functions", flush=True)
        try:
            report["synthetic"].append(synthetic(args.binary, args.work_dir, count))
        except Exception as error:
            report["failures"].append({"synthetic_functions": count, "error": safe_error(error)})
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    try:
        report["accuracy"] = accuracy(args.binary, args.work_dir, Path(__file__).resolve().parents[1])
    except Exception as error:
        report["failures"].append({"accuracy": "failed", "error": safe_error(error)})
    for number, source in enumerate(args.real, 1):
        print(f"Measuring authorized read-only repository case {number}", flush=True)
        try:
            report["real"].append(real_scan(args.binary, args.work_dir, source, number))
        except Exception as error:
            report["failures"].append({"real_case": number, "error": safe_error(error)})
    args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"Completed {len(report['synthetic'])} synthetic and {len(report['real'])} real cases; {len(report['failures'])} failures", flush=True)


if __name__ == "__main__":
    main()
