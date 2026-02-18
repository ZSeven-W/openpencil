import { Link } from '@tanstack/react-router'

import { useState } from 'react'
import { Home, Menu, X } from 'lucide-react'
import { Button } from '@/components/ui/button'

export default function Header() {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <>
      <header className="p-4 flex items-center bg-card text-foreground border-b border-border">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setIsOpen(true)}
          aria-label="Open menu"
        >
          <Menu size={20} />
        </Button>
        <h1 className="ml-4 text-xl font-semibold">
          <Link to="/">
            <img
              src="/tanstack-word-logo-white.svg"
              alt="TanStack Logo"
              className="h-10"
            />
          </Link>
        </h1>
      </header>

      <aside
        className={`fixed top-0 left-0 h-full w-80 bg-background text-foreground shadow-2xl z-50 transform transition-transform duration-300 ease-in-out flex flex-col ${
          isOpen ? 'translate-x-0' : '-translate-x-full'
        }`}
      >
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-xl font-bold">Navigation</h2>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setIsOpen(false)}
            aria-label="Close menu"
          >
            <X size={20} />
          </Button>
        </div>

        <nav className="flex-1 p-4 overflow-y-auto">
          <Link
            to="/"
            onClick={() => setIsOpen(false)}
            className="flex items-center gap-3 p-3 rounded-lg hover:bg-accent transition-colors mb-2"
            activeProps={{
              className:
                'flex items-center gap-3 p-3 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors mb-2',
            }}
          >
            <Home size={20} />
            <span className="font-medium">Home</span>
          </Link>
        </nav>
      </aside>
    </>
  )
}
