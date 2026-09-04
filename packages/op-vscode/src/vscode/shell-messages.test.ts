import { describe, expect, test } from "bun:test";
import {
  isShellControl,
  isShellSaveRequest,
  parseShellCopyText,
  parseShellExternalUrl,
  parseShellReadyOrigin,
} from "./shell-messages";

test("isShellControl matches only a top-level op-shell/ type", () => {
  expect(isShellControl(JSON.stringify({ type: "op-shell/ready", origin: "x" }))).toBe(true);
  expect(isShellControl(JSON.stringify({ type: "op-bridge/snapshot-result", requestId: "r" }))).toBe(false);
});

test("isShellControl does NOT drop a snapshot whose docJson embeds op-shell/", () => {
  // Regression: a legitimate snapshot-result carrying "op-shell/" inside its
  // docJson must reach the session — a substring check would drop it and hang
  // the awaiting save/backup.
  const msg = JSON.stringify({
    type: "op-bridge/snapshot-result",
    requestId: "r1",
    docJson: '{"text":"see op-shell/ready docs"}',
    generation: 2,
    revision: 1,
  });
  expect(msg).toContain("op-shell/");
  expect(isShellControl(msg)).toBe(false);
});

test("isShellControl rejects non-string / non-JSON / typeless payloads", () => {
  expect(isShellControl(123)).toBe(false);
  expect(isShellControl("not json")).toBe(false);
  expect(isShellControl(JSON.stringify({ origin: "x" }))).toBe(false);
  expect(isShellControl(JSON.stringify({ type: 42 }))).toBe(false);
});

test("parseShellReadyOrigin extracts the origin from a ready message", () => {
  expect(parseShellReadyOrigin(JSON.stringify({ type: "op-shell/ready", origin: "vscode-webview://x" }))).toBe(
    "vscode-webview://x",
  );
  expect(parseShellReadyOrigin(JSON.stringify({ type: "op-shell/ready" }))).toBeUndefined();
  expect(parseShellReadyOrigin(JSON.stringify({ type: "op-bridge/opened", generation: 1 }))).toBeUndefined();
  expect(parseShellReadyOrigin("not json")).toBeUndefined();
});

describe("parseShellCopyText", () => {
  test("extracts the text of an op-shell/copy control message", () => {
    expect(parseShellCopyText(JSON.stringify({ type: "op-shell/copy", text: "cfg" }))).toBe("cfg");
  });
  test("returns undefined for other messages and malformed payloads", () => {
    expect(parseShellCopyText(JSON.stringify({ type: "op-shell/ready", origin: "o" }))).toBeUndefined();
    expect(parseShellCopyText(JSON.stringify({ type: "op-shell/copy", text: 5 }))).toBeUndefined();
    expect(parseShellCopyText("not json")).toBeUndefined();
    expect(parseShellCopyText(42)).toBeUndefined();
  });
  test("a business message embedding the literal text is not copy traffic", () => {
    expect(
      parseShellCopyText(JSON.stringify({ type: "op-bridge/opened", note: "op-shell/copy" })),
    ).toBeUndefined();
  });
});

describe("isShellSaveRequest", () => {
  test("matches only the op-shell/save control type", () => {
    expect(isShellSaveRequest(JSON.stringify({ type: "op-shell/save" }))).toBe(true);
    expect(isShellSaveRequest(JSON.stringify({ type: "op-shell/copy", text: "x" }))).toBe(false);
    expect(isShellSaveRequest("not json")).toBe(false);
    expect(isShellSaveRequest(1)).toBe(false);
  });
});

describe("parseShellExternalUrl", () => {
  test("accepts HTTPS and canonicalizes it", () => {
    expect(
      parseShellExternalUrl(
        JSON.stringify({
          type: "op-shell/open-external",
          url: "https://sso.zseven.cn/device?code=abc",
        }),
      ),
    ).toBe("https://sso.zseven.cn/device?code=abc");
  });

  test("accepts loopback HTTP for a development SSO", () => {
    expect(
      parseShellExternalUrl(
        JSON.stringify({ type: "op-shell/open-external", url: "http://127.0.0.1:3100/device" }),
      ),
    ).toBe("http://127.0.0.1:3100/device");
    expect(
      parseShellExternalUrl(
        JSON.stringify({ type: "op-shell/open-external", url: "http://localhost:3100/device" }),
      ),
    ).toBe("http://localhost:3100/device");
  });

  test("rejects unsafe schemes, remote plaintext HTTP, and credentials", () => {
    for (const url of [
      "javascript:alert(1)",
      "file:///tmp/secret",
      "http://sso.zseven.cn/device",
      "https://user:password@sso.zseven.cn/device",
    ]) {
      expect(
        parseShellExternalUrl(JSON.stringify({ type: "op-shell/open-external", url })),
      ).toBeUndefined();
    }
  });

  test("rejects malformed, foreign, and oversized messages", () => {
    expect(parseShellExternalUrl("not json")).toBeUndefined();
    expect(
      parseShellExternalUrl(
        JSON.stringify({ type: "op-bridge/opened", url: "https://example.com" }),
      ),
    ).toBeUndefined();
    expect(
      parseShellExternalUrl(
        JSON.stringify({
          type: "op-shell/open-external",
          url: `https://example.com/${"x".repeat(4_096)}`,
        }),
      ),
    ).toBeUndefined();
  });
});
