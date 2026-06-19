import { describe, it, expect } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { mount } from '@vue/test-utils';
import { makeMockViewer } from './mock-core.js';
import { provideViewer, useViewport } from '../src/index.js';

describe('useViewport (vue)', () => {
  it('updates on viewportchange', async () => {
    const { viewer, emit } = makeMockViewer();
    let vpRef: { value: { zoom: number } };
    const Child = defineComponent({ setup() { vpRef = useViewport() as never; return () => h('div'); } });
    const Parent = defineComponent({ setup() { provideViewer(viewer as never); return () => h(Child); } });
    mount(Parent);
    expect(vpRef!.value.zoom).toBe(1);
    viewer.setZoom(4); emit('viewportchange');
    await nextTick();
    expect(vpRef!.value.zoom).toBe(4);
  });
});
