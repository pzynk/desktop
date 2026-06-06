import { ReactNode } from 'react'

interface SettingRowProps {
  title: string
  description: string
  control: ReactNode
  isLast?: boolean
}

export function SettingRow({ title, description, control, isLast }: SettingRowProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '16px 20px',
      borderBottom: isLast ? 'none' : '1px solid var(--border)',
      transition: 'background 0.2s ease',
    }}>
      <div style={{ paddingRight: '20px' }}>
        <div style={{ fontSize: '14px', fontWeight: 500, color: 'var(--text-primary)', marginBottom: '2px' }}>{title}</div>
        <div style={{ fontSize: '12.5px', color: 'var(--text-tertiary)', lineHeight: 1.4 }}>{description}</div>
      </div>
      <div style={{ flexShrink: 0 }}>
        {control}
      </div>
    </div>
  )
}
