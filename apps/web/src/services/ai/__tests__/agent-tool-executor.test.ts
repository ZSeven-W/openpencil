import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AgentToolExecutor } from '../agent-tool-executor';
import { createEmptyDocument, useDocumentStore } from '../../../stores/document-store';

const designGeneratorMocks = vi.hoisted(() => ({
  insertStreamingNode: vi.fn(),
  applyPostStreamingTreeHeuristics: vi.fn(),
}));

const designCanvasOpsMocks = vi.hoisted(() => ({
  insertStreamingNode: vi.fn(),
  resetGenerationRemapping: vi.fn(),
  setGenerationCanvasWidth: vi.fn(),
}));

const designAnimationMocks = vi.hoisted(() => ({
  startNewAnimationBatch: vi.fn(),
  markNodesForAnimation: vi.fn(),
}));

const canvasMocks = vi.hoisted(() => ({
  zoomToFitContent: vi.fn(),
}));

vi.mock('@/services/ai/design-generator', () => designGeneratorMocks);
vi.mock('@/services/ai/design-canvas-ops', () => designCanvasOpsMocks);
vi.mock('@/services/ai/design-animation', () => designAnimationMocks);
vi.mock('@/canvas/skia-engine-ref', () => canvasMocks);

describe('AgentToolExecutor layout flow', () => {
  beforeEach(() => {
    // Start from a document whose active page has NO frames so that
    // plan_layout exercises its "create new root frame" branch instead
    // of reusing the default frame that createEmptyDocument() ships.
    const doc = createEmptyDocument();
    if (doc.pages?.[0]) doc.pages[0].children = [];
    useDocumentStore.setState({
      document: doc,
      fileName: null,
      isDirty: false,
      fileHandle: null,
      filePath: null,
      saveDialogOpen: false,
    });
    vi.clearAllMocks();
  });

  it('creates a root frame on the first plan_layout call', async () => {
    const executor = new AgentToolExecutor('test-session');
    const initialNodeCount = useDocumentStore.getState().getFlatNodes().length;

    const result = await (executor as any).handlePlanLayout({ prompt: 'mobile food app homepage' });
    const nodes = useDocumentStore.getState().getFlatNodes();

    expect(result.success).toBe(true);
    expect((result.data as any).rootFrameId).toBeTruthy();
    expect(nodes).toHaveLength(initialNodeCount + 1);
    expect(nodes.some((node) => node.id === (result.data as any).rootFrameId)).toBe(true);
  });

  it('rejects repeated plan_layout calls without newRoot before content is inserted', async () => {
    const executor = new AgentToolExecutor('test-session');
    const initialNodeCount = useDocumentStore.getState().getFlatNodes().length;

    const first = await (executor as any).handlePlanLayout({ prompt: 'mobile food app homepage' });
    const second = await (executor as any).handlePlanLayout({ prompt: 'mobile food app homepage' });

    expect(first.success).toBe(true);
    expect(second.success).toBe(false);
    expect(second.data).toEqual({ rootFrameId: (first.data as any).rootFrameId });
    expect(second.error).toContain('Use batch_insert or insert_node');
    expect(second.error).toContain('"newRoot": true');
    expect(useDocumentStore.getState().getFlatNodes()).toHaveLength(initialNodeCount + 1);
  });

  it('allows another plan_layout call when newRoot is explicitly requested', async () => {
    const executor = new AgentToolExecutor('test-session');
    const initialNodeCount = useDocumentStore.getState().getFlatNodes().length;

    const first = await (executor as any).handlePlanLayout({ prompt: 'first mobile screen' });
    const second = await (executor as any).handlePlanLayout({
      prompt: 'second mobile screen',
      newRoot: true,
    });

    expect(first.success).toBe(true);
    expect(second.success).toBe(true);
    expect((second.data as any).rootFrameId).not.toBe((first.data as any).rootFrameId);
    expect(useDocumentStore.getState().getFlatNodes()).toHaveLength(initialNodeCount + 2);
  });

  it('marks content as started after batch_insert and keeps rejecting implicit replanning', async () => {
    const executor = new AgentToolExecutor('test-session');
    const first = await (executor as any).handlePlanLayout({ prompt: 'mobile food app homepage' });

    const insertResult = await (executor as any).handleBatchInsert({
      parentId: (first.data as any).rootFrameId,
      nodes: [{ id: 'section-1', type: 'frame', name: 'Section 1' }],
    });
    const secondPlan = await (executor as any).handlePlanLayout({ prompt: 'another layout pass' });

    expect(insertResult.success).toBe(true);
    expect((insertResult.data as any).inserted).toBe(1);
    expect(secondPlan.success).toBe(false);
    expect(secondPlan.error).toContain('Use batch_insert or insert_node');
    expect(designGeneratorMocks.insertStreamingNode).toHaveBeenCalledTimes(1);
  });

  it('marks content as started after insert_node and keeps rejecting implicit replanning', async () => {
    const executor = new AgentToolExecutor('test-session');

    await (executor as any).handlePlanLayout({ prompt: 'mobile food app homepage' });
    const insertResult = await (executor as any).handleInsertNode({
      parent: null,
      data: { type: 'frame', name: 'Card', width: 320, height: 180, children: [] },
    });
    const secondPlan = await (executor as any).handlePlanLayout({ prompt: 'another layout pass' });

    expect(insertResult.success).toBe(true);
    expect((insertResult.data as any).nodesCreated).toBe(1);
    expect(secondPlan.success).toBe(false);
    expect(secondPlan.error).toContain('Use batch_insert or insert_node');
    expect(designCanvasOpsMocks.insertStreamingNode).toHaveBeenCalledTimes(1);
  });
});
