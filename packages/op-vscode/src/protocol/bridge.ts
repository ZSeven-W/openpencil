// `postMessage` bridge protocol codec between the VS Code extension host
// and the wasm-backed web editor. This is a TypeScript mirror of the Rust
// wire codec in `crates/op-editor-core/src/bridge_protocol.rs` — the Rust
// source is authoritative; keep this file in lockstep with it.
//
// Wire format is a JSON STRING on both directions (Rust `parse(raw: &str)`
// / `event_*() -> String`). Inbound parsing must never throw: foreign
// postMessage traffic (e.g. react-devtools) is ignored, not treated as an
// error.
//
// Message `type` values — outbound (extension → page): `op-bridge/init`,
// `op-bridge/theme`, `op-bridge/locale`, `op-bridge/open-document`, `op-bridge/snapshot`,
// `op-bridge/save-committed`, `op-bridge/resolve-conflict`; inbound (page → extension): `op-bridge/ready`,
// `op-bridge/dirty-changed`, `op-bridge/opened`, `op-bridge/snapshot-result`,
// `op-bridge/snapshot-conflict`, `op-bridge/sync-conflict`,
// `op-bridge/conflict-resolved`, `op-bridge/listening`. Field names are
// camelCase (`requestId`, `serverVersion`, `docJson`).

export type BridgeOutboundToPage =
  | { type: "op-bridge/init"; token: string; mcpUrl?: string }
  | { type: "op-bridge/theme"; colorScheme: "light" | "dark" }
  | { type: "op-bridge/locale"; locale: "zh-CN" | "en-US" }
  | { type: "op-bridge/open-document"; json: string }
  | { type: "op-bridge/snapshot"; purpose: "save" | "backup" | "conflict-backup"; requestId: string }
  | { type: "op-bridge/save-committed"; generation: number; revision: number }
  | { type: "op-bridge/resolve-conflict"; mode: "use-local" | "accept-remote"; requestId: string };

export type BridgeInboundFromPage =
  | { type: "op-bridge/listening" }
  | { type: "op-bridge/ready"; generation: number; revision: number }
  | { type: "op-bridge/dirty-changed"; generation: number; revision: number; dirty: boolean }
  | { type: "op-bridge/opened"; generation: number }
  | { type: "op-bridge/snapshot-result"; requestId: string; docJson: string; generation: number; revision: number }
  | { type: "op-bridge/snapshot-conflict"; requestId: string; serverVersion: number }
  | { type: "op-bridge/sync-conflict"; generation: number; revision: number; serverVersion: number }
  | { type: "op-bridge/conflict-resolved"; requestId: string };

/** Serializes an outbound message to the wire JSON-string format. */
export function encodeOutbound(msg: BridgeOutboundToPage): string {
  return JSON.stringify(msg);
}

/** u64 field validation mirroring Rust's `Value::as_u64()`: a JSON number
 *  that is a non-negative safe integer. Rejects negatives, fractions,
 *  strings, and anything past Number.MAX_SAFE_INTEGER. */
function isU64(x: unknown): x is number {
  return typeof x === "number" && Number.isSafeInteger(x) && x >= 0;
}

function isNonEmptyRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Parses a raw inbound `postMessage` payload. Returns null for anything
 *  that isn't a well-formed, known bridge message — foreign traffic (e.g.
 *  react-devtools) must be ignored, never thrown on. Mirror of Rust's
 *  `BridgeInbound::parse -> Option<Self>`. */
export function parseInboundFromPage(raw: unknown): BridgeInboundFromPage | null {
  if (typeof raw !== "string") return null;

  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!isNonEmptyRecord(value)) return null;

  const ty = value["type"];
  if (typeof ty !== "string") return null;

  switch (ty) {
    case "op-bridge/listening":
      return { type: "op-bridge/listening" };
    case "op-bridge/ready": {
      const generation = value["generation"];
      const revision = value["revision"];
      if (!isU64(generation) || !isU64(revision)) return null;
      return { type: "op-bridge/ready", generation, revision };
    }
    case "op-bridge/dirty-changed": {
      const generation = value["generation"];
      const revision = value["revision"];
      const dirty = value["dirty"];
      if (!isU64(generation) || !isU64(revision) || typeof dirty !== "boolean") return null;
      return { type: "op-bridge/dirty-changed", generation, revision, dirty };
    }
    case "op-bridge/opened": {
      const generation = value["generation"];
      if (!isU64(generation)) return null;
      return { type: "op-bridge/opened", generation };
    }
    case "op-bridge/snapshot-result": {
      const requestId = value["requestId"];
      const docJson = value["docJson"];
      const generation = value["generation"];
      const revision = value["revision"];
      if (
        typeof requestId !== "string" ||
        typeof docJson !== "string" ||
        !isU64(generation) ||
        !isU64(revision)
      ) {
        return null;
      }
      return { type: "op-bridge/snapshot-result", requestId, docJson, generation, revision };
    }
    case "op-bridge/snapshot-conflict": {
      const requestId = value["requestId"];
      const serverVersion = value["serverVersion"];
      if (typeof requestId !== "string" || !isU64(serverVersion)) return null;
      return { type: "op-bridge/snapshot-conflict", requestId, serverVersion };
    }
    case "op-bridge/sync-conflict": {
      const generation = value["generation"];
      const revision = value["revision"];
      const serverVersion = value["serverVersion"];
      if (!isU64(generation) || !isU64(revision) || !isU64(serverVersion)) return null;
      return { type: "op-bridge/sync-conflict", generation, revision, serverVersion };
    }
    case "op-bridge/conflict-resolved": {
      const requestId = value["requestId"];
      if (typeof requestId !== "string") return null;
      return { type: "op-bridge/conflict-resolved", requestId };
    }
    default:
      return null;
  }
}
