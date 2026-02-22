import { useMemo } from 'react'
import type { UIKit, KitComponent, ComponentCategory } from '@/types/uikit'
import ComponentBrowserCard from './component-browser-card'

interface ComponentBrowserGridProps {
  kits: UIKit[]
  searchQuery: string
  activeCategory: ComponentCategory | null
  activeKitId: string | null
}

export default function ComponentBrowserGrid({
  kits,
  searchQuery,
  activeCategory,
  activeKitId,
}: ComponentBrowserGridProps) {
  const filteredItems = useMemo(() => {
    const items: { component: KitComponent; kit: UIKit }[] = []
    const query = searchQuery.toLowerCase().trim()

    for (const kit of kits) {
      if (activeKitId && kit.id !== activeKitId) continue

      for (const comp of kit.components) {
        if (activeCategory && comp.category !== activeCategory) continue
        if (query) {
          const nameMatch = comp.name.toLowerCase().includes(query)
          const tagMatch = comp.tags.some((t) => t.includes(query))
          if (!nameMatch && !tagMatch) continue
        }
        items.push({ component: comp, kit })
      }
    }
    return items
  }, [kits, searchQuery, activeCategory, activeKitId])

  if (filteredItems.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm py-8">
        No components found
      </div>
    )
  }

  return (
    <div className="grid grid-cols-3 gap-2 p-2">
      {filteredItems.map(({ component, kit }) => (
        <ComponentBrowserCard
          key={`${kit.id}-${component.id}`}
          component={component}
          kit={kit}
        />
      ))}
    </div>
  )
}
