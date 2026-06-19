import { defineComponent, Fragment, h, onMounted, onUnmounted, ref, shallowRef } from 'vue';
import { createViewer, type OpViewer } from '@zseven-w/op-web-sdk';
import { provideViewerRef } from './use-viewer.js';

export const DesignView = defineComponent({
  name: 'DesignView',
  props: {
    doc: {
      type: [String, Object] as unknown as () => string | Uint8Array,
      default: undefined,
    },
    wasmUrl: { type: String, default: undefined },
  },
  emits: { load: (_viewer: OpViewer) => true },
  setup(props, { emit, slots }) {
    const canvasRef = ref<HTMLCanvasElement | null>(null);

    // Create the ref in setup() so provide() runs at the correct time.
    // Child composables that call useViewer() will receive this ref and
    // watch it; when viewerRef.value becomes non-null they subscribe.
    const viewerRef = shallowRef<OpViewer | null>(null);
    provideViewerRef(viewerRef);

    let cancelled = false;

    onMounted(async () => {
      const canvas = canvasRef.value;
      if (!canvas) return;
      const v = await createViewer({ canvas, doc: props.doc, wasmUrl: props.wasmUrl });
      if (cancelled) {
        // Component was unmounted before the async viewer resolved — discard it.
        v.destroy();
        return;
      }
      // Setting .value triggers watchers in child composables.
      viewerRef.value = v;
      emit('load', v);
    });

    onUnmounted(() => {
      cancelled = true;
      viewerRef.value?.destroy();
      viewerRef.value = null;
    });

    // Render the canvas plus any slotted children (e.g. child components that
    // call useViewport/useDocument to subscribe to viewer events).
    return () =>
      h(Fragment, [
        h('canvas', { ref: canvasRef, style: 'width:100%;height:100%;display:block' }),
        slots.default?.(),
      ]);
  },
});
