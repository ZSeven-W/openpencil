import { useState, useRef, useEffect } from 'react'
import { cn } from '@/lib/utils'

interface ColorPickerProps {
  value: string
  onChange: (color: string) => void
  label?: string
  className?: string
}

export default function ColorPicker({
  value,
  onChange,
  label,
  className,
}: ColorPickerProps) {
  const [hexInput, setHexInput] = useState(value)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    setHexInput(value)
  }, [value])

  const handleHexChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = e.target.value
    setHexInput(v)
    if (/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(v)) {
      onChange(v)
    }
  }

  const handleNativeChange = (
    e: React.ChangeEvent<HTMLInputElement>,
  ) => {
    onChange(e.target.value)
    setHexInput(e.target.value)
  }

  const handleBlur = () => {
    if (!/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(hexInput)) {
      setHexInput(value)
    }
  }

  return (
    <div className={cn('flex items-center gap-1.5', className)}>
      {label && (
        <span className="text-[10px] text-muted-foreground shrink-0">
          {label}
        </span>
      )}
      <div className="flex items-center h-6 bg-secondary rounded border border-transparent hover:border-input focus-within:border-ring transition-colors flex-1">
        <div className="pl-1 shrink-0">
          <input
            type="color"
            value={value.slice(0, 7)}
            onChange={handleNativeChange}
            className="w-4 h-4 rounded border border-input/50 cursor-pointer bg-transparent p-0"
          />
        </div>
        <input
          ref={inputRef}
          type="text"
          value={hexInput}
          onChange={handleHexChange}
          onBlur={handleBlur}
          className="flex-1 bg-transparent text-foreground text-[11px] px-1.5 py-0.5 focus:outline-none font-mono tabular-nums min-w-0"
          placeholder="#000000"
        />
      </div>
    </div>
  )
}
