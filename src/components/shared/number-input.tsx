import { useState, useRef, useCallback, useEffect } from 'react'

interface NumberInputProps {
  value: number
  onChange: (value: number) => void
  min?: number
  max?: number
  step?: number
  label?: string
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
      className={`flex items-center gap-1 ${className}`}
      onMouseDown={handleMouseDown}
    >
      {label && (
        <span className="text-xs text-muted-foreground w-5 cursor-ew-resize select-none">
          {label}
        </span>
      )}
      <input
        type="text"
        value={localValue}
        onChange={(e) => setLocalValue(e.target.value)}
        onBlur={handleBlur}
        onKeyDown={handleKeyDown}
        className="w-full bg-secondary text-foreground text-xs px-1.5 py-1 rounded-md border border-input focus:border-ring focus:outline-none transition-colors"
      />
      {suffix && (
        <span className="text-xs text-muted-foreground">{suffix}</span>
      )}
    </div>
  )
}
