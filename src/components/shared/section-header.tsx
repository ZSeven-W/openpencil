interface SectionHeaderProps {
  title: string
  actions?: React.ReactNode
}

export default function SectionHeader({ title, actions }: SectionHeaderProps) {
  return (
    <div className="flex items-center justify-between h-7">
      <span className="text-[11px] text-muted-foreground">{title}</span>
      {actions && (
        <div className="flex items-center gap-0.5">{actions}</div>
      )}
    </div>
  )
}
