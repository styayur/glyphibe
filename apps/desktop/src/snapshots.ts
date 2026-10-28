// Async IPC results belong to a loaded graph snapshot, not just a stable ID.
export interface SnapshotToken {
  snapshot: number;
  channel: string;
  request: number;
}
export interface SnapshotResult<T> {
  snapshot: number;
  key: string;
  value: T;
}
export class SnapshotRequests {
  private generation = 0;
  private latest = new Map<string, number>();
  advance(): number {
    this.generation++;
    this.latest.clear();
    return this.generation;
  }
  start(channel: string): SnapshotToken {
    const request = (this.latest.get(channel) ?? 0) + 1;
    this.latest.set(channel, request);
    return { snapshot: this.generation, channel, request };
  }
  accepts(token: SnapshotToken): boolean {
    return (
      token.snapshot === this.generation &&
      this.latest.get(token.channel) === token.request
    );
  }
  cancel(token: SnapshotToken): void {
    if (this.accepts(token)) this.latest.set(token.channel, token.request + 1);
  }
}
export function visibleSnapshot<T>(
  result: SnapshotResult<T> | null,
  snapshot: number,
  key: string,
): T | null {
  return result?.snapshot === snapshot && result.key === key
    ? result.value
    : null;
}
export interface LoadOperation {
  readonly id: number;
  readonly kind: "startup" | "scan" | "import";
}
/** Close the gap before React commits its asynchronous busy flag. */
export class LoadOperations {
  private sequence = 0;
  private active: LoadOperation | null = null;
  begin(kind: LoadOperation["kind"]): LoadOperation | null {
    if (this.active) return null;
    this.active = { id: ++this.sequence, kind };
    return this.active;
  }
  current(operation: LoadOperation): boolean {
    return this.active?.id === operation.id;
  }
  pending(): LoadOperation | null {
    return this.active;
  }
  finish(operation: LoadOperation): boolean {
    if (!this.current(operation)) return false;
    this.active = null;
    return true;
  }
}
