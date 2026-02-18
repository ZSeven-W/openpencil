import { cn } from '@/lib/utils'

interface DropdownSelectProps {
  value: string
  options: { value: string; label: string }[]
  onChange: (value: string) => void
  label?: string
  className?: string
}

export default function DropdownSelect({
  value,
  options,
  onChange,
  label,
  className = '',
}: DropdownSelectProps) {
  return (
    <div className={cn('flex items-center gap-1.5', className)}>
      {label && (
        <span className="text-[10px] text-muted-foreground shrink-0">
          {label}
        </span>
      )}
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="flex-1 h-6 bg-secondary text-foreground text-[11px] px-1.5 rounded border border-transparent hover:border-input focus:border-ring focus:outline-none cursor-pointer transition-colors"
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  )
}
