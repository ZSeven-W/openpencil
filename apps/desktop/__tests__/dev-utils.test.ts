import { describe, expect, it } from 'bun:test';

import {
  getDevServerConflictMessage,
  getElectronBinaryPath,
  getElectronSpawnEnv,
} from '../dev-utils';

describe('getElectronBinaryPath', () => {
  it('uses the packaged electron.exe inside dist on Windows', () => {
    expect(getElectronBinaryPath('C:/repo', 'win32')).toBe(
      'C:/repo/node_modules/electron/dist/electron.exe',
    );
  });

  it('uses the .bin shim on non-Windows platforms', () => {
    expect(getElectronBinaryPath('/repo', 'darwin')).toBe('/repo/node_modules/.bin/electron');
  });

  it('removes ELECTRON_RUN_AS_NODE when launching Electron', () => {
    const env = getElectronSpawnEnv({
      PATH: 'x',
      ELECTRON_RUN_AS_NODE: '1',
      FOO: 'bar',
    });

    expect(env).toEqual({
      PATH: 'x',
      FOO: 'bar',
    });
  });

  it('detects when port 3000 is occupied by a non-Vite server', () => {
    expect(
      getDevServerConflictMessage(
        {
          baseReachable: true,
          viteClientReachable: false,
          viteClientStatus: 404,
        },
        3000,
      ),
    ).toContain('Port 3000 is responding');
  });

  it('does not report a conflict when the Vite client is reachable', () => {
    expect(
      getDevServerConflictMessage(
        {
          baseReachable: true,
          viteClientReachable: true,
          viteClientStatus: 200,
        },
        3000,
      ),
    ).toBeNull();
  });
});
