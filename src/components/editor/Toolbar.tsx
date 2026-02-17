import {
  MousePointer2,
  Square,
  Circle,
  Minus,
  Type,
  Frame,
  Hand,
  ZoomIn,
  ZoomOut,
} from 'lucide-react'
import ToolButton from './ToolButton'
import { useCanvasStore } from '@/stores/canvas-store'
import { MIN_ZOOM, MAX_ZOOM } from '@/canvas/canvas-constants'

export default function Toolbar() {
  const zoom = useCanvasStore((s) => s.viewport.zoom)
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)

  const handleZoomIn = () => {
    if (!fabricCanvas) return
    const newZoom = Math.min(MAX_ZOOM, zoom * 1.25)
    const center = fabricCanvas.getCenterPoint()
    fabricCanvas.zoomToPoint(center, newZoom)
    useCanvasStore.getState().setZoom(newZoom)
    fabricCanvas.requestRenderAll()
  }

  const handleZoomOut = () => {
    if (!fabricCanvas) return
    const newZoom = Math.max(MIN_ZOOM, zoom / 1.25)
    const center = fabricCanvas.getCenterPoint()
    fabricCanvas.zoomToPoint(center, newZoom)
    useCanvasStore.getState().setZoom(newZoom)
    fabricCanvas.requestRenderAll()
  }

  const handleZoomReset = () => {
    if (!fabricCanvas) return
    const center = fabricCanvas.getCenterPoint()
    fabricCanvas.zoomToPoint(center, 1)
    useCanvasStore.getState().setZoom(1)
    fabricCanvas.requestRenderAll()
  }

  return (
    <div className="h-10 bg-gray-800 border-b border-gray-700 flex items-center px-2 gap-1 shrink-0">
      {/* Drawing Tools */}
      <div className="flex items-center gap-0.5 border-r border-gray-700 pr-2 mr-1">
        <ToolButton
          tool="select"
          icon={<MousePointer2 size={16} />}
          label="Select"
          shortcut="V"
        />
        <ToolButton
          tool="frame"
          icon={<Frame size={16} />}
          label="Frame"
          shortcut="F"
        />
        <ToolButton
          tool="rectangle"
          icon={<Square size={16} />}
          label="Rectangle"
          shortcut="R"
        />
        <ToolButton
          tool="ellipse"
          icon={<Circle size={16} />}
          label="Ellipse"
          shortcut="O"
        />
        <ToolButton
          tool="line"
          icon={<Minus size={16} />}
          label="Line"
          shortcut="L"
        />
        <ToolButton
          tool="text"
          icon={<Type size={16} />}
          label="Text"
          shortcut="T"
        />
        <ToolButton
          tool="hand"
          icon={<Hand size={16} />}
          label="Hand"
          shortcut="H"
        />
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Zoom Controls */}
      <div className="flex items-center gap-1">
        <button
          type="button"
          onClick={handleZoomOut}
          className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-700 transition-colors"
          title="Zoom Out"
        >
          <ZoomOut size={14} />
        </button>
        <button
          type="button"
          onClick={handleZoomReset}
          className="text-xs text-gray-300 px-1.5 py-0.5 rounded hover:bg-gray-700 transition-colors tabular-nums min-w-[3.5rem] text-center"
          title="Reset Zoom"
        >
          {Math.round(zoom * 100)}%
        </button>
        <button
          type="button"
          onClick={handleZoomIn}
          className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-700 transition-colors"
          title="Zoom In"
        >
          <ZoomIn size={14} />
        </button>
      </div>
    </div>
  )
}
