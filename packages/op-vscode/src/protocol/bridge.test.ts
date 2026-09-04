import { test, expect } from "bun:test";
import { encodeOutbound, parseInboundFromPage, type BridgeInboundFromPage, type BridgeOutboundToPage } from "./bridge";

// ---------------------------------------------------------------------------
// encodeOutbound — one sample per BridgeOutboundToPage variant, round-tripped
// through JSON.parse to confirm the wire shape matches Rust's field names.
// ---------------------------------------------------------------------------

test("encodeOutbound: init", () => {
  const msg: BridgeOutboundToPage = { type: "op-bridge/init", token: "t0k" };
  expect(JSON.parse(encodeOutbound(msg))).toEqual({ type: "op-bridge/init", token: "t0k" });
});

test("encodeOutbound: theme", () => {
  const light: BridgeOutboundToPage = { type: "op-bridge/theme", colorScheme: "light" };
  const dark: BridgeOutboundToPage = { type: "op-bridge/theme", colorScheme: "dark" };
  expect(JSON.parse(encodeOutbound(light))).toEqual({
    type: "op-bridge/theme",
    colorScheme: "light",
  });
  expect(JSON.parse(encodeOutbound(dark))).toEqual({
    type: "op-bridge/theme",
    colorScheme: "dark",
  });
});

test("encodeOutbound: locale", () => {
  const zh: BridgeOutboundToPage = { type: "op-bridge/locale", locale: "zh-CN" };
  const en: BridgeOutboundToPage = { type: "op-bridge/locale", locale: "en-US" };
  expect(JSON.parse(encodeOutbound(zh))).toEqual({
    type: "op-bridge/locale",
    locale: "zh-CN",
  });
  expect(JSON.parse(encodeOutbound(en))).toEqual({
    type: "op-bridge/locale",
    locale: "en-US",
  });
});

test("encodeOutbound: open-document", () => {
  const msg: BridgeOutboundToPage = { type: "op-bridge/open-document", json: '{"a":1}' };
  expect(JSON.parse(encodeOutbound(msg))).toEqual({ type: "op-bridge/open-document", json: '{"a":1}' });
});

test("encodeOutbound: snapshot", () => {
  const msg: BridgeOutboundToPage = { type: "op-bridge/snapshot", purpose: "backup", requestId: "r1" };
  expect(JSON.parse(encodeOutbound(msg))).toEqual({
    type: "op-bridge/snapshot",
    purpose: "backup",
    requestId: "r1",
  });
});

test("encodeOutbound: save-committed", () => {
  const msg: BridgeOutboundToPage = { type: "op-bridge/save-committed", generation: 3, revision: 41 };
  expect(JSON.parse(encodeOutbound(msg))).toEqual({
    type: "op-bridge/save-committed",
    generation: 3,
    revision: 41,
  });
});

test("encodeOutbound: resolve-conflict", () => {
  const msg: BridgeOutboundToPage = { type: "op-bridge/resolve-conflict", mode: "use-local", requestId: "r1" };
  expect(encodeOutbound(msg)).toBe(
    '{"type":"op-bridge/resolve-conflict","mode":"use-local","requestId":"r1"}',
  );
});

// ---------------------------------------------------------------------------
// parseInboundFromPage — one legal sample per BridgeInboundFromPage variant.
// Field names/values copied verbatim from bridge_protocol.rs's event_* output.
// ---------------------------------------------------------------------------

test("parseInboundFromPage: listening", () => {
  const raw = '{"type":"op-bridge/listening"}';
  const expected: BridgeInboundFromPage = { type: "op-bridge/listening" };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

test("parseInboundFromPage: ready", () => {
  const raw = '{"type":"op-bridge/ready","generation":2,"revision":17}';
  const expected: BridgeInboundFromPage = { type: "op-bridge/ready", generation: 2, revision: 17 };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

test("parseInboundFromPage: dirty-changed", () => {
  const raw = '{"type":"op-bridge/dirty-changed","generation":2,"revision":17,"dirty":true}';
  const expected: BridgeInboundFromPage = {
    type: "op-bridge/dirty-changed",
    generation: 2,
    revision: 17,
    dirty: true,
  };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

test("parseInboundFromPage: opened", () => {
  const raw = '{"type":"op-bridge/opened","generation":5}';
  const expected: BridgeInboundFromPage = { type: "op-bridge/opened", generation: 5 };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

test("parseInboundFromPage: snapshot-result", () => {
  const raw = '{"type":"op-bridge/snapshot-result","requestId":"r1","docJson":"{}","generation":2,"revision":17}';
  const expected: BridgeInboundFromPage = {
    type: "op-bridge/snapshot-result",
    requestId: "r1",
    docJson: "{}",
    generation: 2,
    revision: 17,
  };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

test("parseInboundFromPage: snapshot-conflict", () => {
  const raw = '{"type":"op-bridge/snapshot-conflict","requestId":"r1","serverVersion":12}';
  const expected: BridgeInboundFromPage = {
    type: "op-bridge/snapshot-conflict",
    requestId: "r1",
    serverVersion: 12,
  };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

test("parseInboundFromPage: sync-conflict", () => {
  const raw = '{"type":"op-bridge/sync-conflict","generation":2,"revision":5,"serverVersion":12}';
  const expected: BridgeInboundFromPage = {
    type: "op-bridge/sync-conflict",
    generation: 2,
    revision: 5,
    serverVersion: 12,
  };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

test("parseInboundFromPage: conflict-resolved", () => {
  const raw = '{"type":"op-bridge/conflict-resolved","requestId":"r1"}';
  const expected: BridgeInboundFromPage = { type: "op-bridge/conflict-resolved", requestId: "r1" };
  expect(parseInboundFromPage(raw)).toEqual(expected);
});

// ---------------------------------------------------------------------------
// Rejection cases — foreign traffic, malformed payloads, wrong field types.
// Mirrors bridge_protocol.rs::tests::parse_inbound_messages.
// ---------------------------------------------------------------------------

test("parseInboundFromPage: unknown type -> null (react-devtools, etc.)", () => {
  expect(parseInboundFromPage('{"type":"react-devtools"}')).toBeNull();
});

test("parseInboundFromPage: not JSON -> null", () => {
  expect(parseInboundFromPage("not json")).toBeNull();
});

test("parseInboundFromPage: non-string input -> null", () => {
  expect(parseInboundFromPage(undefined)).toBeNull();
  expect(parseInboundFromPage(null)).toBeNull();
  expect(parseInboundFromPage(42)).toBeNull();
  expect(parseInboundFromPage({ type: "op-bridge/opened", generation: 1 })).toBeNull();
  expect(parseInboundFromPage(["op-bridge/opened"])).toBeNull();
});

test("parseInboundFromPage: JSON array or scalar -> null", () => {
  expect(parseInboundFromPage("[1,2,3]")).toBeNull();
  expect(parseInboundFromPage("42")).toBeNull();
  expect(parseInboundFromPage('"a string"')).toBeNull();
  expect(parseInboundFromPage("null")).toBeNull();
});

test("parseInboundFromPage: missing type field -> null", () => {
  expect(parseInboundFromPage('{"generation":1}')).toBeNull();
});

test("parseInboundFromPage: non-string type field -> null", () => {
  expect(parseInboundFromPage('{"type":123}')).toBeNull();
});

test("parseInboundFromPage: missing required field -> null", () => {
  expect(parseInboundFromPage('{"type":"op-bridge/ready","generation":2}')).toBeNull();
  expect(parseInboundFromPage('{"type":"op-bridge/opened"}')).toBeNull();
  expect(parseInboundFromPage('{"type":"op-bridge/conflict-resolved"}')).toBeNull();
});

test("parseInboundFromPage: wrong field type -> null", () => {
  // generation as a string, not a number.
  expect(parseInboundFromPage('{"type":"op-bridge/opened","generation":"5"}')).toBeNull();
  // dirty as a number, not a boolean.
  expect(
    parseInboundFromPage('{"type":"op-bridge/dirty-changed","generation":1,"revision":1,"dirty":1}'),
  ).toBeNull();
  // requestId as a number, not a string.
  expect(
    parseInboundFromPage('{"type":"op-bridge/conflict-resolved","requestId":7}'),
  ).toBeNull();
});

test("parseInboundFromPage: u64 field rejects negative, fractional, non-numeric", () => {
  expect(parseInboundFromPage('{"type":"op-bridge/opened","generation":-1}')).toBeNull();
  expect(parseInboundFromPage('{"type":"op-bridge/opened","generation":1.5}')).toBeNull();
  expect(parseInboundFromPage('{"type":"op-bridge/opened","generation":null}')).toBeNull();
  expect(parseInboundFromPage('{"type":"op-bridge/opened","generation":true}')).toBeNull();
  // Past Number.MAX_SAFE_INTEGER — Rust's u64 range exceeds JS's safe-integer
  // range, so values beyond it must be rejected rather than silently rounded.
  expect(
    parseInboundFromPage(`{"type":"op-bridge/opened","generation":${Number.MAX_SAFE_INTEGER + 2}}`),
  ).toBeNull();
  // Zero is a valid u64.
  expect(parseInboundFromPage('{"type":"op-bridge/opened","generation":0}')).toEqual({
    type: "op-bridge/opened",
    generation: 0,
  });
});

// ---------------------------------------------------------------------------
// docJson escaped-quote round trip (snapshot-result payload carries an
// embedded JSON document as a string, mirroring
// bridge_protocol.rs::tests::snapshot_result_json_escapes_doc_payload).
// ---------------------------------------------------------------------------

test("parseInboundFromPage: snapshot-result docJson round-trips escaped quotes", () => {
  const innerDoc = '{"a":"b \\" c"}';
  const raw = JSON.stringify({
    type: "op-bridge/snapshot-result",
    requestId: "r1",
    docJson: innerDoc,
    generation: 2,
    revision: 17,
  });
  const parsed = parseInboundFromPage(raw);
  expect(parsed).toEqual({
    type: "op-bridge/snapshot-result",
    requestId: "r1",
    docJson: innerDoc,
    generation: 2,
    revision: 17,
  });
  // The inner docJson string itself parses back into the original object.
  expect(JSON.parse((parsed as { docJson: string }).docJson)).toEqual({ a: 'b " c' });
});
