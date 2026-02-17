import { createFileRoute, Link } from '@tanstack/react-router'
import { PenTool, Plus } from 'lucide-react'

export const Route = createFileRoute('/')({
  component: LandingPage,
  head: () => ({
    meta: [{ title: 'OpenPencil - Design as Code' }],
  }),
})

function LandingPage() {
  return (
    <div className="min-h-screen bg-gray-900 flex flex-col items-center justify-center text-white">
      <div className="text-center mb-12">
        <div className="flex items-center justify-center gap-3 mb-4">
          <PenTool size={40} className="text-blue-400" />
          <h1 className="text-5xl font-bold tracking-tight">
            Open
            <span className="text-blue-400">Pencil</span>
          </h1>
        </div>
        <p className="text-xl text-gray-400">
          Open-source vector design tool. Design as Code.
        </p>
      </div>

      <div className="flex gap-4">
        <Link
          to="/editor"
          className="flex items-center gap-2 px-6 py-3 bg-blue-500 hover:bg-blue-600 text-white font-medium rounded-lg transition-colors shadow-lg shadow-blue-500/25"
        >
          <Plus size={18} />
          New Design
        </Link>
      </div>

      <p className="mt-8 text-sm text-gray-500">
        Press{' '}
        <kbd className="px-1.5 py-0.5 bg-gray-800 rounded text-gray-400 text-xs">
          Ctrl
        </kbd>
        {' + '}
        <kbd className="px-1.5 py-0.5 bg-gray-800 rounded text-gray-400 text-xs">
          N
        </kbd>
        {' '}to create a new design
      </p>
    </div>
  )
}
