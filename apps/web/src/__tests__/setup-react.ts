/**
 * Vitest setup file to fix "multiple copies of React" error in bun monorepos.
 *
 * Problem: Vite's ESM module runner creates a separate React instance (via its
 * transform pipeline) that has different ReactSharedInternals than the native
 * CJS React used by react-dom. When react-dom renders and sets the hook dispatcher
 * on CJS React's ReactSharedInternals.H, hooks in pen-react (which use the
 * vite-transformed React) see a null dispatcher and throw "Invalid hook call".
 *
 * Root cause: `import 'react'` through vite's pipeline returns a module with its
 * own `__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE` object,
 * separate from the one that native CJS `require('react')` returns (which react-dom
 * uses internally via its own require chain).
 *
 * Fix: In setupFiles (which run in the same vitest worker scope as each test file),
 * install a proxy on the vite-transformed React's internals so all reads/writes
 * delegate to the native CJS React's internals. After this, when react-dom sets H
 * (the hook dispatcher), pen-react hooks see it immediately.
 */
import { createRequire } from 'node:module';

// Import 'react' via vite's transform pipeline — same instance that pen-react hooks use
import * as viteTranformedReact from 'react';

// Load react via native CJS require — same instance that react-dom uses internally.
// Resolve dynamically via node's own module lookup so this file is not tied to
// any single developer's machine (the previous hardcoded absolute path broke
// the entire apps/web test suite on every checkout outside that user's home).
const require = createRequire(import.meta.url);
const cjsReact = require('react') as Record<string, any>;

const viteInternals = (viteTranformedReact as any)
  .__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE as Record<string, any>;
const cjsInternals =
  cjsReact.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE as Record<string, any>;

if (viteInternals && cjsInternals && viteInternals !== cjsInternals) {
  // Make vite-transformed React's internals delegate all reads/writes to CJS internals.
  // This bridges the two React instances so react-dom's dispatcher is visible to hooks.
  for (const key of Object.keys(cjsInternals)) {
    Object.defineProperty(viteInternals, key, {
      get: () => (cjsInternals as any)[key],
      set: (v) => {
        (cjsInternals as any)[key] = v;
      },
      configurable: true,
      enumerable: true,
    });
  }
}
