import type { InjectionKey, ShallowRef } from 'vue';
import type { OpViewer } from '@zseven-w/op-web-sdk';
export const viewerKey: InjectionKey<ShallowRef<OpViewer | null>> = Symbol('op-web-sdk-viewer');
