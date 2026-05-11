export const MAX_CLOUD_FILE_PAYLOAD_BYTES = 50 * 1024 * 1024;

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex++;
  }
  return `${value >= 10 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
}

export function createCloudFilePayloadTooLargeMessage(
  sizeBytes: number | undefined,
  maxBytes = MAX_CLOUD_FILE_PAYLOAD_BYTES,
): string {
  const size = typeof sizeBytes === 'number' ? `${formatBytes(sizeBytes)}; ` : '';
  return `This design is too large to send to the cloud API (${size}limit ${formatBytes(maxBytes)}). Export .op for now, or remove embedded images before saving to cloud.`;
}
