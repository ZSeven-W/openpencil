// Pure classification of the shell's control messages (op-shell/*). Kept
// vscode-free and testable: a mis-classification here can drop a legitimate
// snapshot whose docJson happens to contain the text "op-shell/", hanging the
// save/backup that awaits it — so the match must be on the parsed top-level
// `type`, never a substring of the raw payload.

/** True only when `raw` is a JSON string whose top-level `type` is an
 *  op-shell/* control type. Business messages (op-bridge/*) and any payload
 *  that merely embeds "op-shell/" inside its data are NOT control traffic. */
export function isShellControl(raw: unknown): boolean {
  if (typeof raw !== "string") return false;
  try {
    const v = JSON.parse(raw) as { type?: unknown };
    return typeof v.type === "string" && v.type.startsWith("op-shell/");
  } catch {
    return false;
  }
}

/** True for an op-shell/save control message — the embedded editor
 *  forwarding a Cmd/Ctrl+S it saw inside the cross-origin iframe (the
 *  workbench cannot observe those keystrokes itself). */
export function isShellSaveRequest(raw: unknown): boolean {
  if (typeof raw !== "string") return false;
  try {
    return (JSON.parse(raw) as { type?: unknown }).type === "op-shell/save";
  } catch {
    return false;
  }
}

/** The text payload of an op-shell/copy control message, else undefined.
 *  The embedded editor relays clipboard writes through the extension host
 *  because the webview's nested-iframe permissions chain rejects
 *  `navigator.clipboard` writes inside the editor iframe. */
export function parseShellCopyText(raw: unknown): string | undefined {
  if (typeof raw !== "string") return undefined;
  try {
    const v = JSON.parse(raw) as { type?: unknown; text?: unknown };
    if (v.type === "op-shell/copy" && typeof v.text === "string") return v.text;
  } catch {
    /* not a control message */
  }
  return undefined;
}

/**
 * Parse a request from the embedded editor to open an authentication or help
 * page in the user's browser.
 *
 * The extension host is the security boundary here: the daemon page is
 * allowed to request HTTPS URLs, plus loopback HTTP for an explicitly
 * configured local development SSO. Other schemes, remote plaintext HTTP,
 * credential-bearing URLs, and unreasonably large values are rejected before
 * they reach `vscode.env.openExternal`.
 */
export function parseShellExternalUrl(raw: unknown): string | undefined {
  if (typeof raw !== "string") return undefined;
  let value: { type?: unknown; url?: unknown };
  try {
    value = JSON.parse(raw) as { type?: unknown; url?: unknown };
  } catch {
    return undefined;
  }
  if (value.type !== "op-shell/open-external" || typeof value.url !== "string") {
    return undefined;
  }
  if (value.url.length === 0 || value.url.length > 4_096) return undefined;

  try {
    const url = new URL(value.url);
    if (url.username !== "" || url.password !== "") return undefined;
    const loopback =
      url.hostname === "127.0.0.1" ||
      url.hostname === "localhost" ||
      url.hostname === "[::1]";
    if (url.protocol !== "https:" && !(url.protocol === "http:" && loopback)) {
      return undefined;
    }
    return url.toString();
  } catch {
    return undefined;
  }
}

/** The origin reported by an op-shell/ready control message, else undefined. */
export function parseShellReadyOrigin(raw: unknown): string | undefined {
  if (typeof raw !== "string") return undefined;
  try {
    const v = JSON.parse(raw) as { type?: unknown; origin?: unknown };
    if (v.type === "op-shell/ready" && typeof v.origin === "string") return v.origin;
  } catch {
    /* not a control message */
  }
  return undefined;
}
