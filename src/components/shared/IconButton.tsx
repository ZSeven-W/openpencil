import type { ReactNode, ButtonHTMLAttributes } from 'react'

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: ReactNode
  label: string
  active?: boolean
  size?: 'sm' | 'md'
}

export default function IconButton({
  icon,
  label,
  active = false,
  size = 'md',
  className = '',
  ...props
}: IconButtonProps) {
  const sizeClass = size === 'sm' ? 'p-1' : 'p-1.5'
  const activeClass = active
    ? 'bg-blue-500 text-white'
    : 'text-gray-400 hover:bg-gray-700 hover:text-white'

  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      className={`${sizeClass} rounded transition-colors ${activeClass} ${className}`}
      {...props}
    >
      {icon}
    </button>
  )
}
