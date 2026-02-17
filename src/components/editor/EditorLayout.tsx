import { lazy, Suspense } from 'react'
import Toolbar from './Toolbar'
import LayerPanel from '@/components/panels/LayerPanel'
import PropertyPanel from '@/components/panels/PropertyPanel'

const FabricCanvas = lazy(() => import('@/canvas/FabricCanvas'))

export default function EditorLayout() {
  return (
    <div className="h-screen flex flex-col bg-gray-900">
      <Toolbar />
      <div className="flex-1 flex overflow-hidden">
        <LayerPanel />
        <Suspense
          fallback={
            <div className="flex-1 flex items-center justify-center bg-neutral-100 text-gray-400">
              Loading canvas...
            </div>
          }
        >
          <FabricCanvas />
        </Suspense>
        <PropertyPanel />
      </div>
    </div>
  )
}
