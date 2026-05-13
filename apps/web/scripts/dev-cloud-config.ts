export function buildDevCloudWebEnv(
  env: NodeJS.ProcessEnv = process.env,
): NodeJS.ProcessEnv {
  return {
    ...env,
    OPENPENCIL_CODEGEN_WORKER: 'disabled',
    OPENPENCIL_DEV_CLOUD: '1',
    VITE_OPENPENCIL_CODEGEN_WORKER: 'disabled',
  };
}
