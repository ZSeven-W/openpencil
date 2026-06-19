import { inject, provide, shallowRef, type ShallowRef } from 'vue';
import type { OpViewer } from '@zseven-w/op-web-sdk';
import { viewerKey } from './injection.js';

/** Provide a viewer synchronously (manual / test use). */
export function provideViewer(viewer: OpViewer): void {
  provide(viewerKey, shallowRef(viewer));
}

/**
 * Provide a pre-created ShallowRef so that DesignView can fill it
 * asynchronously after mount while still calling provide() in setup().
 */
export function provideViewerRef(ref: ShallowRef<OpViewer | null>): void {
  provide(viewerKey, ref);
}

/**
 * Return the injected ShallowRef<OpViewer | null>. Consumers read `.value`
 * to access the viewer; it becomes non-null once the async viewer is ready.
 * Throws if called outside a DesignView / provideViewer context.
 */
export function useViewer(): ShallowRef<OpViewer | null> {
  const v = inject(viewerKey);
  if (!v) throw new Error('op-web-sdk-vue: useViewer must be used inside <DesignView>/provideViewer');
  return v;
}
