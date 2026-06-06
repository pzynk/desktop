interface SectionHeaderProps {
  title: string
  description: string
}

export function SectionHeader({ title, description }: SectionHeaderProps) {
  return (
    <div style={{ marginBottom: '16px', paddingLeft: '4px' }}>
      <h3 style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-primary)', margin: 0 }}>{title}</h3>
      <p style={{ fontSize: '13px', color: 'var(--text-tertiary)', marginTop: '4px', margin: 0 }}>{description}</p>
    </div>
  )
}
