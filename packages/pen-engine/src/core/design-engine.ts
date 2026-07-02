import type {
  PenDocument,
  PenNode,
  ToolType,
  DesignEngineOptions,
  DesignEngineEvents,
  VariableDefinition,
} from '@zseven-w/pen-types';
import { TypedEventEmitter } from './event-emitter.js';
import { HistoryManager } from './history-manager.js';
import { DocumentManager } from './document-manager.js';
import { SelectionManager } from './selection-manager.js';
import { PageManager } from './page-manager.js';
import { VariableManager } from './variable-manager.js';
import { ViewportController } from './viewport-controller.js';
import { EngineSpatialIndex } from './spatial-index.js';
import { parseSvgToNodes } from './svg-parser.js';

/**
 * DesignEngine -- headless design engine with zero DOM/React/Zustand dependencies.
 *
 * Composes internal managers for document, history, selection, pages, variables,
 * viewport, and spatial indexing. All getters return immutable references.
 * Mutations increment the version counter and emit typed events.
 *
 * Usage:
 * ```typescript
 * const engine = new DesignEngine();
 * engine.loadDocument(doc);
 * engine.addNode(null, node);
 * engine.on('document:change', (doc) => console.log('changed'));
 * const code = engine.generateCode('react');
 * ```
 */
export class DesignEngine {
  private emitter = new TypedEventEmitter<DesignEngineEvents>();
  private historyManager: HistoryManager;
  private documentManager: DocumentManager;
  private selectionManager: SelectionManager;
  private pageManager: PageManager;
  private variableManager: VariableManager;
  private viewportController: ViewportController;
  private spatialIndex: EngineSpatialIndex;
  private _version = 0;
  private _activeTool: ToolType = 'select';
  private _batchDepth = 0;
  private _batchPendingChange = false;
  private options: DesignEngineOptions;

  constructor(options?: DesignEngineOptions) {
    this.options = options ?? {};

    this.historyManager = new HistoryManager({
      maxStates: this.options.maxHistoryStates,
      onChange: (state) => this.emitter.emit('history:change', state),
    });

    this.documentManager = new DocumentManager({
      historyManager: this.historyManager,
      onChange: (doc) => this.onDocumentMutated(doc),
      getActivePageId: () => this.pageManager?.getActivePage() ?? null,
    });

    this.selectionManager = new SelectionManager({
      onChange: (ids) => {
        this._version++;
        this.emitter.emit('selection:change', ids);
      },
      onHover: (id) => this.emitter.emit('node:hover', id),
    });

    this.pageManager = new PageManager({
      getDocument: () => this.documentManager.getDocument(),
      setDocument: (doc) => {
        // Page mutations go through history
        this.historyManager.push(this.documentManager.getDocument());
        // Direct assignment to avoid double-push via DocumentManager.mutate
        (this.documentManager as any).document = doc;
        this.onDocumentMutated(doc);
      },
      onPageChange: (pageId) => {
        this._version++;
        this.emitter.emit('page:change', pageId);
      },
    });

    this.variableManager = new VariableManager({
      getDocument: () => this.documentManager.getDocument(),
      setDocument: (doc) => {
        this.historyManager.push(this.documentManager.getDocument());
        (this.documentManager as any).document = doc;
        this.onDocumentMutated(doc);
      },
    });

    this.viewportController = new ViewportController({
      onChange: (state) => {
        this._version++;
        this.emitter.emit('viewport:change', state);
      },
    });

    this.spatialIndex = new EngineSpatialIndex();
  }

  dispose(): void {
    this.emitter.dispose();
  }

  // ── Snapshot ──

  get version(): number {
    return this._version;
  }

  // ── Document ──

  loadDocument(doc: PenDocument): void {
    this.documentManager.loadDocument(doc);
    this._version++;
  }

  getDocument(): PenDocument {
    return this.documentManager.getDocument();
  }

  createDocument(): PenDocument {
    return this.documentManager.createDocument();
  }

  // ── Node CRUD ──

  addNode(parentId: string | null, node: PenNode, index?: number): void {
    this.documentManager.addNode(parentId, node, index);
  }

  updateNode(id: string, updates: Partial<PenNode>): void {
    this.documentManager.updateNode(id, updates);
  }

  removeNode(id: string): void {
    this.documentManager.removeNode(id);
  }

  moveNode(id: string, newParentId: string | null, index: number): void {
    this.documentManager.moveNode(id, newParentId, index);
  }

  duplicateNode(id: string): string | null {
    return this.documentManager.duplicateNode(id);
  }

  groupNodes(ids: string[]): string | null {
    return this.documentManager.groupNodes(ids);
  }

  ungroupNode(groupId: string): void {
    this.documentManager.ungroupNode(groupId);
  }

  getNodeById(id: string): PenNode | undefined {
    return this.documentManager.getNodeById(id);
  }

  // ── Pages ──

  addPage(): string {
    return this.pageManager.addPage();
  }

  removePage(pageId: string): void {
    this.pageManager.removePage(pageId);
  }

  setActivePage(pageId: string): void {
    this.pageManager.setActivePage(pageId);
  }

  getActivePage(): string {
    return this.pageManager.getActivePage();
  }

  // ── Selection ──

  select(ids: string[]): void {
    this.selectionManager.select(ids);
  }

  clearSelection(): void {
    this.selectionManager.clearSelection();
  }

  getSelection(): string[] {
    return this.selectionManager.getSelection();
  }

  // ── Hover ──

  getHoveredId(): string | null {
    return this.selectionManager.getHoveredId();
  }

  setHoveredId(id: string | null): void {
    this.selectionManager.setHoveredId(id);
  }

  // ── Viewport ──

  get zoom(): number {
    return this.viewportController.zoom;
  }
  get panX(): number {
    return this.viewportController.panX;
  }
  get panY(): number {
    return this.viewportController.panY;
  }

  setViewport(zoom: number, panX: number, panY: number): void {
    this.viewportController.setViewport(zoom, panX, panY);
  }

  zoomToRect(
    x: number,
    y: number,
    w: number,
    h: number,
    containerW: number,
    containerH: number,
  ): void {
    this.viewportController.zoomToRect(x, y, w, h, containerW, containerH);
  }

  /**
   * Compute the bounding box of all nodes on the active page.
   * Returns null if the page is empty.
   * Used by DesignCanvas to zoom-to-fit on initial load.
   */
  getContentBounds(): { x: number; y: number; w: number; h: number } | null {
    const children = this.documentManager.getActivePageChildren();
    if (!children.length) return null;
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    for (const node of children) {
      const nx = node.x ?? 0;
      const ny = node.y ?? 0;
      const nw = (node as any).width ?? 0;
      const nh = (node as any).height ?? 0;
      minX = Math.min(minX, nx);
      minY = Math.min(minY, ny);
      maxX = Math.max(maxX, nx + nw);
      maxY = Math.max(maxY, ny + nh);
    }
    if (!isFinite(minX)) return null;
    return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
  }

  screenToScene(x: number, y: number): { x: number; y: number } {
    return this.viewportController.screenToScene(x, y);
  }

  sceneToScreen(x: number, y: number): { x: number; y: number } {
    return this.viewportController.sceneToScreen(x, y);
  }

  // ── Hit testing ──

  hitTest(x: number, y: number): PenNode | null {
    return this.spatialIndex.hitTestNode(x, y);
  }

  searchRect(x: number, y: number, w: number, h: number): PenNode[] {
    return this.spatialIndex.searchRectNodes(x, y, w, h);
  }

  // ── Tool state ──

  setActiveTool(tool: ToolType): void {
    this._activeTool = tool;
    this._version++;
    this.emitter.emit('tool:change', tool);
  }

  getActiveTool(): ToolType {
    return this._activeTool;
  }

  // ── History ──

  undo(): void {
    const restored = this.historyManager.undo(this.getDocument());
    if (restored) {
      (this.documentManager as any).document = restored;
      this._version++;
      this.emitter.emit('document:change', restored);
    }
  }

  redo(): void {
    const restored = this.historyManager.redo(this.getDocument());
    if (restored) {
      (this.documentManager as any).document = restored;
      this._version++;
      this.emitter.emit('document:change', restored);
    }
  }

  get canUndo(): boolean {
    return this.historyManager.canUndo;
  }

  get canRedo(): boolean {
    return this.historyManager.canRedo;
  }

  // ── Batch ──

  batch(fn: () => void): void {
    this._batchDepth++;
    this._batchPendingChange = false;
    this.historyManager.startBatch(this.getDocument());
    try {
      fn();
    } finally {
      this._batchDepth--;
      this.historyManager.endBatch(this.getDocument());
      if (this._batchDepth === 0 && this._batchPendingChange) {
        this._batchPendingChange = false;
        this._version++;
        this.emitter.emit('document:change', this.getDocument());
      }
    }
  }

  // ── Variables ──

  setVariable(name: string, def: VariableDefinition): void {
    this.variableManager.setVariable(name, def);
  }

  removeVariable(name: string): void {
    this.variableManager.removeVariable(name);
  }

  renameVariable(oldName: string, newName: string): void {
    this.variableManager.renameVariable(oldName, newName);
  }

  resolveVariable(ref: string): unknown {
    return this.variableManager.resolveVariable(ref);
  }

  // ── Import/Export ──

  importSVG(svg: string, parentId?: string): PenNode[] {
    const nodes = parseSvgToNodes(svg);
    for (const node of nodes) {
      this.addNode(parentId ?? null, node);
    }
    return nodes;
  }

  importFigma(_buffer: ArrayBuffer): PenDocument {
    // Dynamic import to avoid bundling pen-figma in headless scenarios
    // that don't use Figma import.
    throw new Error(
      'importFigma requires the pen-figma package. Use: import { parseFigFile, figmaToPenDocument } from "@zseven-w/pen-figma"',
    );
  }

  // ── Events ──

  on<K extends keyof DesignEngineEvents>(event: K, cb: DesignEngineEvents[K]): () => void {
    return this.emitter.on(event, cb);
  }

  off<K extends keyof DesignEngineEvents>(event: K, cb: DesignEngineEvents[K]): void {
    this.emitter.off(event, cb);
  }

  // ── Internal ──

  /** Called when the document is mutated by any manager. */
  private onDocumentMutated(doc: PenDocument): void {
    if (this._batchDepth > 0) {
      this._batchPendingChange = true;
      return;
    }
    this._version++;
    this.emitter.emit('document:change', doc);
  }

  /**
   * Access to the spatial index for browser adapter.
   * Not part of the public API spec, but needed internally.
   */
  getSpatialIndex(): EngineSpatialIndex {
    return this.spatialIndex;
  }

  /** Access to viewport controller for browser adapter. */
  getViewportController(): ViewportController {
    return this.viewportController;
  }
}
