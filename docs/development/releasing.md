# Releasing a reviewed version

The [gate ledger](release-gates.md) and supporting evidence control approval.
A successful build or CI run alone cannot approve a Stable release. Maintainers
must inspect the full native lifecycle, security, provenance and performance
results. Never replace historical releases or force-push Git history.

1. Review the implementation and claims on a feature PR. Complete Rust
   format/Clippy/tests, frontend typecheck/lint/unit/E2E, CLI/SDK/Agent checks,
   boundary review, license inventory, target audits and reproducible benchmarks.
2. Build the CLI and NSIS desktop installer from committed reviewed source using
   `scripts/build-windows-release.ps1 -TargetDirectory TARGET`. The script remaps
   workspace, user and target paths in Rust diagnostics, scans both executables
   for the original private prefixes, verifies actual native packaging tools and
   writes hash-bound `glyphibe-build-provenance.json` outside the source tree.
   Use an external Cargo target where local disk capacity requires it. The
   Windows CI job uses this same script and uploads complete candidate assets;
   record its actual hosted image, compiler and Visual Studio product metadata.
   Set `CARGO_HOME` to an isolated generic build-cache directory outside the user,
   workspace and target prefixes: native C compiler diagnostics are not covered
   by Rust's remapping. CI uses the hosted runner's generic temporary directory.
   The script rejects a source-cache location that could retain a private prefix.
3. Actually install/upgrade the NSIS package. Open the public fixture, test
   initial camera framing, sphere selection/orbit, all four modes, source,
   teaching, synchronized controls, window close/reopen, uninstall and reinstall.
   Record executable/installer hashes, actual outcomes and genuine screenshots.
   Check bundled original legal-file hashes. Do not modify analyzed projects.
4. Finish the documentation and evidence, pass all required checks and merge
   through the protected branch. Verify main is synchronized and clean, and
   its final CI results pass. Source versions must agree before packaging.
5. Run `scripts/package-release.ps1` with the reviewed Cargo target directory,
   a **new empty external output directory**, and the stable numeric version.
   The script refuses uncommitted source, records the source SHA and hashes,
   includes original notices in every ZIP, and produces `SHA256SUMS`.
   It requires matching source/binary provenance and includes the sanitized
   `build-provenance.json`. Download the CI candidate artifact for native
   validation when the hosted runner is the distribution build environment.
   Upload only top-level files; `stage-*` directories are local packaging inputs.
6. Create an annotated source tag and a **draft** release with the real assets.
   This is publication preparation, not Stable approval. Re-download every asset
   to another empty directory. Do not verify cached upload inputs as downloads.
7. Compute the SHA-256 of the **local packaging** `SHA256SUMS` and CLI executable
   as independent trust anchors. Run the following verification against the
   downloaded directory and the matching source checkout:

```text
python scripts/verify_release_assets.py --directory DOWNLOADED --version 1.0.0 --source-commit REVIEWED_SHA --trusted-sums-sha256 LOCAL_SUMS_SHA --run-cli --trusted-cli-sha256 LOCAL_CLI_SHA --output EXTERNAL_REPORT_JSON
```

The script verifies full top-level coverage, every hash/size, source identity,
exact ZIP member sets, legal-file and executable payload hashes, traversal and
symlink rejection. The hash-pinned Windows execution then checks version/help,
doctor JSON and a real offline fixture analysis/search with no API keys and
unreachable HTTP proxies. It never extracts archives or executes analyzed code.
The verification report stays outside the downloaded directory so it cannot
silently change checksum coverage.

8. Compare the downloaded installer hash with the installer actually used for
   lifecycle validation. Record the remote verification outcome and source SHA
   in release notes. Only after every necessary gate passes, publish as Latest,
   with both Draft and Prerelease false. Check the remote release flags and
   public assets again.

Windows artifacts are unsigned unless a securely managed certificate was
explicitly configured and the actual signatures verified. Do not promise that
SmartScreen warnings will be absent. The current CLI requires Microsoft's
Visual C++ v14 x64 Runtime; the desktop requires WebView2. Runtime prerequisites
and verified platforms belong in the installation and release notes.

Checksums detect distribution changes relative to the trusted packaging inputs;
they are not code signatures or proof that a binary was reproducibly built.
