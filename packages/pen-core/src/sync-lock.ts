/**
 * When locked, document-store → Fabric sync is skipped (Fabric is the source).
 *
 * Uses a getter function instead of a bare `let` export so that cross-module
 * reads always resolve the current value — even if the bundler does not
 * preserve ES-module live bindings for `let` variables.
 */
let _locked = false

export function isFabricSyncLocked(): boolean {
  return _locked
}

export function setFabricSyncLock(v: boolean) {
  _locked = v
}
