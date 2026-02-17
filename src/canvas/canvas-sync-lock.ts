/** When true, document-store → Fabric sync is skipped (Fabric is the source). */
export let fabricSyncLock = false

export function setFabricSyncLock(v: boolean) {
  fabricSyncLock = v
}
