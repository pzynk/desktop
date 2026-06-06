import { useState, useEffect } from 'react'
import { X } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'

interface SettingsModalProps {
  onClose: () => void
}

export function SettingsModal({ onClose }: SettingsModalProps) {
  const [ip, setIp] = useState<string | null>(null)
  const [name, setName] = useState<string | null>(null)
  const [version, setVersion] = useState<string>('…')

  useEffect(() => {
    invoke<string>('get_device_ip').then(setIp).catch(() => {})
    invoke<string>('get_device_name').then(setName).catch(() => {})
    getVersion().then(setVersion).catch(() => {})
  }, [])

  return (
    <div className="modal-backdrop">
      <div className="modal-card" style={{ maxWidth: 440, width: '90%' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 }}>
          <div className="modal-title" style={{ margin: 0 }}>Settings</div>
          <button className="btn btn-ghost btn-icon" onClick={onClose} style={{ borderRadius: '50%' }}>
            <X size={16} strokeWidth={2.5} />
          </button>
        </div>
        <div className="modal-sub" style={{ marginBottom: 20 }}>
          Desktop app configuration and local network details.
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div style={{
            background: 'var(--bg-base)',
            border: '1px solid var(--border)',
            borderRadius: 'var(--radius-md)',
            overflow: 'hidden',
          }}>
            {[
              { label: 'Device Name', value: name ?? '…' },
              { label: 'Local IP', value: ip ?? '…' },
              { label: 'Discovery Port', value: '7890' },
              { label: 'TCP Port', value: '7891' },
              { label: 'Version', value: version },
              { label: 'Protocol', value: 'TCP + UDP Discovery' },
            ].map((row, i, arr) => (
              <div key={row.label} style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '12px 16px',
                borderBottom: i < arr.length - 1 ? '1px solid var(--border)' : 'none',
              }}>
                <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{row.label}</span>
                <span style={{
                  fontSize: 12.5,
                  fontFamily: 'JetBrains Mono, monospace',
                  color: 'var(--text-primary)',
                }}>{row.value}</span>
              </div>
            ))}
          </div>
        </div>
        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 24 }}>
          <button className="btn btn-primary" style={{ background: 'var(--text-primary)', color: 'var(--bg-base)', boxShadow: 'none' }} onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  )
}
