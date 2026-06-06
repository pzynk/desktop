interface ToggleProps {
  enabled: boolean
  onToggle: () => void
  id: string
}

export function Toggle({ enabled, onToggle, id }: ToggleProps) {
  return (
    <button
      id={id}
      onClick={onToggle}
      style={{
        display: 'flex',
        alignItems: 'center',
        padding: 0,
        borderRadius: '999px',
        background: 'transparent',
        border: 'none',
        cursor: 'pointer',
        WebkitTapHighlightColor: 'transparent',
      }}
    >
      <div style={{
        width: 48,
        height: 28,
        borderRadius: 14,
        background: enabled ? 'var(--success)' : 'var(--bg-elevated)',
        border: `1px solid ${enabled ? 'var(--success)' : 'var(--border)'}`,
        position: 'relative',
        transition: 'all 0.25s cubic-bezier(0.4, 0, 0.2, 1)',
        boxShadow: enabled ? '0 0 12px rgba(52, 211, 153, 0.2)' : 'inset 0 2px 4px rgba(0,0,0,0.1)',
      }}>
        <div style={{
          position: 'absolute',
          top: 2,
          left: enabled ? 22 : 2,
          width: 22,
          height: 22,
          borderRadius: '50%',
          background: '#fff',
          transition: 'all 0.25s cubic-bezier(0.4, 0, 0.2, 1)',
          boxShadow: '0 2px 5px rgba(0,0,0,0.2)',
        }} />
      </div>
    </button>
  )
}
