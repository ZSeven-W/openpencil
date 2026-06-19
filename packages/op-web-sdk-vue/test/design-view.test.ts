import { describe, it, expect, vi, beforeEach } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { mount, flushPromises } from '@vue/test-utils';
import { makeMockViewer } from './mock-core.js';
const mock = makeMockViewer();
vi.mock('@zseven-w/op-web-sdk', () => ({ createViewer: vi.fn(async () => mock.viewer) }));
import { DesignView } from '../src/index.js';
import { useViewport } from '../src/index.js';
import { createViewer } from '@zseven-w/op-web-sdk';

describe('DesignView (vue)', () => {
  beforeEach(() => vi.clearAllMocks());

  it('creates a viewer on mount, emits load, destroys on unmount', async () => {
    const wrapper = mount(DesignView, { props: { doc: '{"version":"1.0"}' } });
    expect(wrapper.find('canvas').exists()).toBe(true);
    await flushPromises();
    expect(createViewer).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('load')?.[0]?.[0]).toBe(mock.viewer);
    wrapper.unmount();
    expect(mock.viewer.destroy).toHaveBeenCalled();
  });

  it('child composable inside DesignView receives viewport after async load', async () => {
    // Proves that provide() in setup() + async fill lets children inject correctly.
    let vpRef: ReturnType<typeof useViewport> | undefined;
    const Child = defineComponent({
      setup() {
        vpRef = useViewport();
        return () => h('div');
      },
    });
    const Parent = defineComponent({
      setup() {
        return () => h(DesignView, { doc: '{"version":"1.0"}' }, { default: () => h(Child) });
      },
    });

    mount(Parent);
    // Before load: viewport is null (viewer not yet created).
    expect(vpRef!.value).toBeNull();

    // After async createViewer resolves, viewerRef is set and watcher fires.
    await flushPromises();
    await nextTick();
    expect(vpRef!.value).not.toBeNull();
    expect((vpRef!.value as { zoom: number }).zoom).toBe(1);
  });
});
