import { describe, it, expect } from 'vitest';
import { defineComponent, h } from 'vue';
import { mount } from '@vue/test-utils';
import { makeMockViewer } from './mock-core.js';
import { provideViewer, useViewer } from '../src/index.js';

describe('useViewer (vue)', () => {
  it('returns a ref whose .value is the provided viewer', () => {
    const { viewer } = makeMockViewer();
    let got: unknown;
    // useViewer() now returns ShallowRef<OpViewer | null>; read .value to get the viewer.
    const Child = defineComponent({ setup() { got = useViewer().value; return () => h('div'); } });
    const Parent = defineComponent({ setup() { provideViewer(viewer as never); return () => h(Child); } });
    mount(Parent);
    expect(got).toBe(viewer);
  });
});
