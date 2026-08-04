import { useState } from 'react'
import { Terminal, Copy, Check, X } from 'lucide-react'

export interface MissingDependency {
  feature: string
  title: string
  package: string
  command: string
  description: string
}

interface DependencyModalProps {
  dependency: MissingDependency
  onClose: () => void
}

export function DependencyModal({ dependency, onClose }: DependencyModalProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = () => {
    navigator.clipboard.writeText(dependency.command)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="modal-backdrop">
      <div className="modal-card" style={{ width: '480px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '12px' }}>
          <div style={{
            width: 40,
            height: 40,
            borderRadius: 'var(--radius-md)',
            background: 'rgba(239, 68, 68, 0.15)',
            color: 'var(--danger)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: 0
          }}>
            <Terminal width={20} height={20} />
          </div>
          <div>
            <div className="modal-title" style={{ fontSize: '17px', margin: 0 }}>
              {dependency.title}
            </div>
            <div style={{ fontSize: '12px', color: 'var(--text-tertiary)', marginTop: '2px' }}>
              Package: <code style={{ color: 'var(--accent)', background: 'var(--bg-base)', padding: '2px 6px', borderRadius: '4px' }}>{dependency.package}</code>
            </div>
          </div>
        </div>

        <div style={{ fontSize: '13.5px', color: 'var(--text-secondary)', marginBottom: '16px', lineHeight: '1.5' }}>
          {dependency.description}
        </div>

        <div style={{ marginBottom: '20px' }}>
          <div style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-tertiary)', marginBottom: '6px' }}>
            RUN THIS COMMAND IN YOUR LINUX TERMINAL:
          </div>
          <div style={{
            position: 'relative',
            background: 'var(--bg-base)',
            border: '1px solid var(--border)',
            borderRadius: 'var(--radius-md)',
            padding: '12px 40px 12px 14px',
            fontFamily: 'monospace',
            fontSize: '12px',
            color: 'var(--accent)',
            wordBreak: 'break-all',
            lineHeight: '1.4'
          }}>
            {dependency.command}
            <button
              onClick={handleCopy}
              style={{
                position: 'absolute',
                top: '8px',
                right: '8px',
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border)',
                borderRadius: '4px',
                padding: '4px 6px',
                color: 'var(--text-secondary)',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center'
              }}
              title="Copy Command"
            >
              {copied ? <Check size={14} color="#10b981" /> : <Copy size={14} />}
            </button>
          </div>
        </div>

        <div className="modal-actions" style={{ marginTop: '0' }}>
          <button className="btn btn-ghost" onClick={onClose}>
            <X size={16} /> Close
          </button>
          <button
            className="btn btn-primary"
            style={{ background: 'var(--accent)', color: '#fff' }}
            onClick={handleCopy}
          >
            {copied ? <Check size={16} /> : <Copy size={16} />} {copied ? 'Copied!' : 'Copy Command'}
          </button>
        </div>
      </div>
    </div>
  )
}
