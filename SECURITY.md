# Security policy

Do not post secrets, private code or exploitable details in public issues.
Use [GitHub private vulnerability reporting](https://github.com/styayur/glyphibe/security/advisories/new).
Until a stable release exists, no version is represented as production-supported.

The analyzer treats repositories as data. It does not run their programs,
install dependencies, execute package scripts, launch language servers or send
source to a model. Git is invoked only for read-only revision metadata with
separate arguments. File discovery does not follow symlinks; source retrieval
and cache storage validate containment and reject linked/reparse paths.

Knowledge imports are an explicit network action, bounded to validated public
GitHub repository endpoints with redirects disabled. Text remains untrusted
local data and never becomes instructions, executable code or a redistributable
default knowledge entry. The offline core has no network requirement.

Tauri exposes selected-repository operations without a shell plugin. MCP is
stdio-only, reads one chosen index and enables isolated proposal writes only
when the owner passes `--allow-proposals`. It does not edit source or replace
observed facts. The host/Agent is responsible for protecting its local session.

Static analysis is incomplete for reflection, dynamic dispatch, dependency
injection and unproven cross-language bridges. Unresolved diagnostics are part
of the result. Reachability is not a vulnerability verdict or failure probability.
Path checks reduce accidental access; the application is not a sandbox for
executing hostile software. It never offers that execution as an analysis step.
