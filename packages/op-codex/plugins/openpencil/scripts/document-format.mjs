import { isDeepStrictEqual } from 'node:util';

// Match the Rust schema's structural image visitor. Never rewrite free-form
// user state, text, action parameters or variables that resemble image refs.
export function liveDocument(disk) {
  const copy = structuredClone(disk);
  const table = copy.images || {};
  delete copy.images; delete copy.imageThumbs; delete copy.editorMeta;
  const resolve = value => typeof value === 'string' && value.startsWith('op-image:') &&
    Object.hasOwn(table, value.slice(9)) ? table[value.slice(9)] : value;
  const fills = list => {
    if (Array.isArray(list)) for (const fill of list) if (fill?.type === 'image') fill.url = resolve(fill.url);
  };
  const style = node => { fills(node.fill); fills(node.stroke?.fill); };
  const stack = [...(copy.children || []), ...(copy.pages || [])];
  while (stack.length) {
    const node = stack.pop();
    if (!node || typeof node !== 'object') continue;
    if (node.type === 'image') node.src = resolve(node.src);
    style(node);
    for (const override of Object.values(node.states || {})) if (override && typeof override === 'object') style(override);
    if (Array.isArray(node.children)) stack.push(...node.children);
  }
  return copy;
}

export function snapshotMatches(disk, live) {
  const meta = disk.editorMeta;
  return isDeepStrictEqual(liveDocument(disk), liveDocument(live.document)) &&
    (!meta || (meta.activePageIndex === live.activePageIndex &&
      meta.preserveAuthoredGeometry === live.preserveAuthoredGeometry &&
      (meta.scenario ?? null) === (live.scenario ?? null)));
}

export function diskDocument(live) {
  return { ...live.document, editorMeta: { activePageIndex: live.activePageIndex,
    preserveAuthoredGeometry: live.preserveAuthoredGeometry,
    ...(live.scenario != null ? { scenario: live.scenario } : {}) } };
}
