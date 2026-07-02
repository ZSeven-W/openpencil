/**
 * Re-export from converters/ modules for backwards compatibility.
 * @deprecated Import directly from './converters/index.js' in new code.
 */
export type { ConversionContext, IconLookupResult } from './converters/index.js';
export {
  convertNode,
  convertChildren,
  collectImageBlobs,
  setIconLookup,
} from './converters/index.js';
