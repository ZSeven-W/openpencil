import { useState, useRef, useCallback, useEffect } from 'react'
import { cn } from '@/lib/utils'

interface NumberInputProps {
  value: number
  onChange: (value: number) => void
  min?: number
  max?: number
  step?: number
  label?: string
  icon?: React.ReactNode
  suffix?: string
  className?: string
}

export default function NumberInput({
  value,
  onChange,
  min,
  max,
  step = 1,
  label,
  icon,
  suffix,
  className = '',
}: NumberInputProps) {
  const [localValue, setLocalValue] = useState(String(value))
  const [isDragging, setIsDragging] = useState(false)
  const dragStartY = useRef(0)
  const dragStartValue = useRef(0)

  useEffect(() => {
    if (!isDragging) {
      setLocalValue(String(Math.round(value * 100) / 100))
    }
  }, [value, isDragging])

  const clamp = useCallback(
    (v: number) => {
      let result = v
      if (min !== undefined) result = Math.max(min, result)
      if (max !== undefined) result = Math.min(max, result)
      return result
    },
    [min, max],
  )

  const handleBlur = () => {
    const parsed = parseFloat(localValue)
    if (!isNaN(parsed)) {
      onChange(clamp(parsed))
    } else {
      setLocalValue(String(value))
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleBlur()
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      onChange(clamp(value + step))
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      onChange(clamp(value - step))
    }
  }

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.target instanceof HTMLInputElement) return
    setIsDragging(true)
    dragStartY.current = e.clientY
    dragStartValue.current = value

    const handleMouseMove = (ev: MouseEvent) => {
      const delta = dragStartY.current - ev.clientY
      const newValue = clamp(dragStartValue.current + delta * step)
      onChange(newValue)
    }

    const handleMouseUp = () => {
      setIsDragging(false)
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  return (
    <div
      className={cn(
        'flex items-center h-6 bg-secondary rounded border border-transparent',
        'hover:border-input focus-within:border-ring transition-colors',
        className,
      )}
      onMouseDown={handleMouseDown}
    >
      {label && (
        <span className="text-[10px] text-muted-foreground pl-1.5 pr-0.5 cursor-ew-resize select-none shrink-0">
          {label}
        </span>
      )}
      {icon && (
        <span className="pl-1 pr-0.5 text-muted-foreground cursor-ew-resize select-none shrink-0 [&_svg]:w-3 [&_svg]:h-3">
          {icon}
        </span>
      )}
      <input
        type="text"
        value={localValue}
        onChange={(e) => setLocalValue(e.target.value)}
        onBlur={handleBlur}
        onKeyDown={handleKeyDown}
        className="w-full bg-transparent text-foreground text-[11px] px-1 py-0.5 focus:outline-none tabular-nums"
      />
      {suffix && (
        <span className="text-[10px] text-muted-foreground pr-1.5 shrink-0">
          {suffix}
        </span>
      )}
    </div>
  )
}
