# Native query snapshot isolation

Stable node IDs intentionally survive repository changes. They therefore cannot
identify a cached query result's graph snapshot. Git HEAD can also remain the
same while working files change.

The frontend advances an in-memory generation whenever a graph is loaded,
including a different repository, an imported graph, or a rescan of the same
working revision. Advancement invalidates outstanding requests synchronously,
before React effects or older promise microtasks can commit data. Cache entries
also carry the generation and their selected-node/control key, so an old cache
entry cannot appear during the render before a new effect runs.

Impact, source and explanation requests use independent channels. Each channel
rejects superseded responses and cancelled effects. A failed current request
does not restore a prior snapshot's cached result. Source reads depend on the
node and snapshot; changing explanation depth or language does not needlessly
reread source. Explanations additionally key their depth and language.

The regression tests resolve deferred replies after switching graphs with the
same stable ID, rescan the same working revision, reject a new query, and deliver
older teaching replies after a newer teaching request. These test asynchronous
cache isolation; they do not substitute for Rust analysis or native lifecycle
validation.

Native state publishes the repository root and graph together under one mutex.
Starting analysis is serialized against snapshot reads; impact, explanation and
source queries reject with an explicit analysis-in-progress error until the scan
finishes. A scan guard releases the busy gate on every return path. Readers can
therefore never pair a new repository root with an old graph.

The frontend also serializes startup, folder scans and graph imports with a
synchronous operation token. Repeated Ctrl+O cannot create a second request, and
an older completion/finally cannot clear a newer operation's busy state. Imports
wait until a native load finishes or is cancelled; cancelled import reads cannot
publish a graph later. Beginning a load invalidates query results immediately,
and the evidence panel is hidden while loading.
