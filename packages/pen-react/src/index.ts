// Context
export { DesignEngineContext } from './context.js';

// Provider
export { DesignProvider } from './provider.js';
export type { DesignProviderProps } from './provider.js';

// Hooks
export { useDesignEngine } from './hooks/use-design-engine.js';
export { useDocument } from './hooks/use-document.js';
export { useSelection } from './hooks/use-selection.js';
export { useViewport } from './hooks/use-viewport.js';
export { useActiveTool } from './hooks/use-active-tool.js';
export { useHistory } from './hooks/use-history.js';
export { useActiveNode } from './hooks/use-active-node.js';
export { useActivePage } from './hooks/use-active-page.js';
export { useHover } from './hooks/use-hover.js';
export { useVariables } from './hooks/use-variables.js';

// Utilities
export { useEngineSubscribe } from './utils/use-engine-subscribe.js';

// Components
export { DesignCanvas } from './components/design-canvas.js';
export type { DesignCanvasProps } from './components/design-canvas.js';

// Task 8: CoreToolbar
export { CoreToolbar } from './components/core-toolbar.js';
export type { CoreToolbarProps } from './components/core-toolbar.js';
export { ToolButton } from './components/tool-button.js';
export { ShapeToolDropdown } from './components/shape-tool-dropdown.js';
export type { ShapeToolDropdownProps } from './components/shape-tool-dropdown.js';

// Task 9: LayerPanel
export { LayerPanel } from './components/layer-panel.js';
export { LayerItem } from './components/layer-item.js';
export type { LayerItemProps, DropPosition } from './components/layer-item.js';
export { LayerContextMenu } from './components/layer-context-menu.js';
export type { LayerContextMenuProps } from './components/layer-context-menu.js';

// Task 10: PropertyPanel + Sections
export { PropertyPanel } from './components/property-panel.js';
export type { PropertyPanelProps } from './components/property-panel.js';
export { SizeSection } from './components/sections/size-section.js';
export { FillSection } from './components/sections/fill-section.js';
export { StrokeSection } from './components/sections/stroke-section.js';
export { TextSection } from './components/sections/text-section.js';
export { TextLayoutSection } from './components/sections/text-layout-section.js';
export { CornerRadiusSection } from './components/sections/corner-radius-section.js';
export { EffectsSection } from './components/sections/effects-section.js';
export { LayoutSection } from './components/sections/layout-section.js';
export { LayoutPaddingSection } from './components/sections/layout-padding-section.js';
export { AppearanceSection } from './components/sections/appearance-section.js';
export { IconSection } from './components/sections/icon-section.js';
export { ImageSection } from './components/sections/image-section.js';
export { ExportSection } from './components/sections/export-section.js';

// Task 11: Shared UI Components
export { ColorPicker } from './components/color-picker.js';
export type { ColorPickerProps } from './components/color-picker.js';
export { NumberInput } from './components/number-input.js';
export type { NumberInputProps } from './components/number-input.js';
export { SectionHeader } from './components/section-header.js';
export { FontPicker } from './components/font-picker.js';
export type { FontPickerProps, FontInfo } from './components/font-picker.js';
export { VariablePicker } from './components/variable-picker.js';
export type { VariablePickerProps } from './components/variable-picker.js';
export { IconPickerDialog } from './components/icon-picker-dialog.js';
export type { IconPickerDialogProps, IconPickerPosition } from './components/icon-picker-dialog.js';

// Task 12: BooleanToolbar, PageTabs, StatusBar
export { BooleanToolbar } from './components/boolean-toolbar.js';
export { PageTabs } from './components/page-tabs.js';
export type { PageTabsProps } from './components/page-tabs.js';
export { StatusBar } from './components/status-bar.js';
export type { StatusBarProps } from './components/status-bar.js';

// Stores
export { useUIStore } from './stores/ui-store.js';
export type { UIStoreState } from './stores/ui-store.js';
