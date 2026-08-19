/** Raw shape of `op_editor_account_snapshot`'s JSON. */
interface AccountSnapshotPayload {
  signed_in?: boolean;
  display_name?: string;
  username?: string;
  primary_email?: string;
}

/**
 * Signed-in profile snapshot decoded from the engine's JSON account snapshot
 * (`editorAccountSnapshot`). The private auth runtime never exposes its device
 * token to the shell — this is everything the shell is allowed to know.
 * Ported from `AccountCenterOverlay.kt`'s `AccountSnapshot`.
 */
export class AccountSnapshot {
  readonly signedIn: boolean;
  readonly displayName: string | null;
  readonly username: string | null;
  readonly primaryEmail: string | null;

  private constructor(
    signedIn: boolean,
    displayName: string | null,
    username: string | null,
    primaryEmail: string | null
  ) {
    this.signedIn = signedIn;
    this.displayName = displayName;
    this.username = username;
    this.primaryEmail = primaryEmail;
  }

  static parse(json: string | null): AccountSnapshot | null {
    if (json === null || json.length === 0) {
      return null;
    }
    try {
      const payload: AccountSnapshotPayload = JSON.parse(json) as AccountSnapshotPayload;
      return new AccountSnapshot(
        payload.signed_in === true,
        AccountSnapshot.text(payload.display_name),
        AccountSnapshot.text(payload.username),
        AccountSnapshot.text(payload.primary_email)
      );
    } catch (_error) {
      return null;
    }
  }

  /** Best display label; null when the engine reports no identity at all. */
  label(): string | null {
    if (this.displayName !== null) {
      return this.displayName;
    }
    return this.username;
  }

  detail(): string {
    if (this.primaryEmail !== null) {
      return this.primaryEmail;
    }
    return this.username ?? '';
  }

  private static text(value: string | undefined): string | null {
    if (typeof value !== 'string' || value.length === 0) {
      return null;
    }
    return value;
  }
}
