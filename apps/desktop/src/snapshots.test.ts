import { expect, it } from "vitest";
import { LoadOperations, SnapshotRequests, visibleSnapshot } from "./snapshots";
import type { SnapshotResult } from "./snapshots";
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
it("serializes startup, repeated open shortcuts and imports before React can paint busy", () => {
  const loads = new LoadOperations(),
    startup = loads.begin("startup")!;
  expect(loads.begin("scan")).toBeNull();
  expect(loads.begin("import")).toBeNull();
  expect(loads.finish(startup)).toBe(true);
  const scan = loads.begin("scan")!;
  expect(loads.begin("scan")).toBeNull();
  expect(loads.finish(startup)).toBe(false);
  expect(loads.pending()).toBe(scan);
  expect(loads.finish(scan)).toBe(true);
  expect(loads.begin("import")).not.toBeNull();
});
it("cannot publish a cancelled import or let its old finally clear a newer scan", async () => {
  const loads = new LoadOperations(),
    old = loads.begin("import")!,
    file = deferred<string>();
  let loaded = "original graph";
  const settled = file.promise
    .then((value) => {
      if (loads.current(old)) loaded = value;
    })
    .finally(() => loads.finish(old));
  loads.finish(old);
  const scan = loads.begin("scan")!;
  file.resolve("late imported graph");
  await settled;
  expect(loaded).toBe("original graph");
  expect(loads.current(scan)).toBe(true);
  expect(loads.pending()).toBe(scan);
});

it.each([
  "different repository with the same ID",
  "rescan of the same working revision",
])("invalidates all IPC channels for %s", async () => {
  const requests = new SnapshotRequests();
  const before = requests.advance();
  const results = new Map<string, SnapshotResult<string>>();
  const pending = ["impact", "source", "explanation"].map((channel) => {
    const token = requests.start(channel),
      reply = deferred<string>();
    const settled = reply.promise.then((value) => {
      if (requests.accepts(token))
        results.set(channel, {
          snapshot: token.snapshot,
          key: "stable-function",
          value,
        });
    });
    return { token, reply, settled };
  });
  const after = requests.advance();
  expect(after).not.toBe(before);
  for (const request of pending) request.reply.resolve("old graph result");
  await Promise.all(pending.map((p) => p.settled));
  expect(results.size).toBe(0);
  expect(
    visibleSnapshot(
      { snapshot: before, key: "stable-function", value: "cached impact" },
      after,
      "stable-function",
    ),
  ).toBeNull();
  const current = requests.start("impact");
  expect(requests.accepts(current)).toBe(true);
  expect(
    visibleSnapshot(
      {
        snapshot: current.snapshot,
        key: "stable-function",
        value: "new graph result",
      },
      after,
      "stable-function",
    ),
  ).toBe("new graph result");
});
it("does not restore an old cached impact when the new snapshot request fails", async () => {
  const requests = new SnapshotRequests();
  const old = requests.advance();
  let result: SnapshotResult<string> | null = {
    snapshot: old,
    key: "same-id",
    value: "old calls",
  };
  const current = requests.advance(),
    token = requests.start("impact"),
    reply = deferred<string>();
  expect(visibleSnapshot(result, current, "same-id")).toBeNull();
  const settled = reply.promise
    .then((value) => {
      if (requests.accepts(token))
        result = { snapshot: token.snapshot, key: "same-id", value };
    })
    .catch(() => {
      if (requests.accepts(token)) result = null;
    });
  reply.reject(new Error("query failed"));
  await settled;
  expect(visibleSnapshot(result, current, "same-id")).toBeNull();
});
it("isolates late explanations and cancelled requests without cancelling a source read", () => {
  const requests = new SnapshotRequests();
  const snapshot = requests.advance(),
    source = requests.start("source"),
    beginner = requests.start("explanation");
  const expert = requests.start("explanation");
  expect(requests.accepts(beginner)).toBe(false);
  expect(requests.accepts(expert)).toBe(true);
  expect(requests.accepts(source)).toBe(true);
  requests.cancel(expert);
  expect(requests.accepts(expert)).toBe(false);
  expect(
    visibleSnapshot(
      { snapshot, key: "id:1:en", value: "beginner answer" },
      snapshot,
      "id:4:zh",
    ),
  ).toBeNull();
});
