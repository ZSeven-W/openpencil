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
    <div className={`flex items-center gap-2 ${className}`}>
      {label && (
        <span className="text-xs text-gray-400">{label}</span>
      )}
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="flex-1 bg-gray-700 text-white text-xs px-1.5 py-1 rounded border border-gray-600 focus:border-blue-500 focus:outline-none cursor-pointer"
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
