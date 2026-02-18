import { useState, useRef, useEffect } from 'react'

interface ColorPickerProps {
  value: string
  onChange: (color: string) => void
  label?: string
}

export default function ColorPicker({
  value,
  onChange,
  label,
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
    <div className="flex items-center gap-2">
      {label && (
        <span className="text-xs text-muted-foreground">{label}</span>
      )}
      <div className="relative">
        <input
          type="color"
          value={value.slice(0, 7)}
          onChange={handleNativeChange}
          className="w-6 h-6 rounded-md border border-input cursor-pointer bg-transparent p-0"
        />
      </div>
      <input
        ref={inputRef}
        type="text"
        value={hexInput}
        onChange={handleHexChange}
        onBlur={handleBlur}
        className="flex-1 bg-secondary text-foreground text-xs px-1.5 py-1 rounded-md border border-input focus:border-ring focus:outline-none font-mono transition-colors"
        placeholder="#000000"
      />
    </div>
  )
}
