import { useState, useEffect } from 'react'
import { X } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart'

interface SettingsModalProps {
  onClose: () => void
}

export function SettingsModal({ onClose }: SettingsModalProps) {
  const [ip, setIp] = useState<string | null>(null)
  const [name, setName] = useState<string | null>(null)
  const [version, setVersion] = useState<string>('…')
  const [autostart, setAutostart] = useState<boolean | null>(null)

  useEffect(() => {
    invoke<string>('get_device_ip').then(setIp).catch(() => {})
    invoke<string>('get_device_name').then(setName).catch(() => {})
    getVersion().then(setVersion).catch(() => {})
    isEnabled().then(setAutostart).catch(() => {})
  }, [])

  const toggleAutostart = async () => {
    if (autostart === null) return
    try {
      if (autostart) {
        await disable()
        setAutostart(false)
      } else {
        await enable()
        setAutostart(true)
      }
    } catch (err) {
      console.error('Failed to toggle autostart:', err)
    }
  }

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
            ].map((row) => (
              <div key={row.label} style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '12px 16px',
                borderBottom: '1px solid var(--border)',
              }}>
                <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{row.label}</span>
                <span style={{
                  fontSize: 12.5,
                  fontFamily: 'JetBrains Mono, monospace',
                  color: 'var(--text-primary)',
                }}>{row.value}</span>
              </div>
            ))}

            <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '12px 16px',
            }}>
              <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>Run at Startup</span>
              <button
                onClick={toggleAutostart}
                className="btn"
                style={{
                  padding: '4px 10px',
                  fontSize: 12,
                  fontWeight: 600,
                  background: autostart ? 'rgba(34,197,94,0.15)' : 'rgba(239,68,68,0.15)',
                  color: autostart ? 'var(--success)' : 'var(--danger)',
                  border: `1px solid ${autostart ? 'rgba(34,197,94,0.3)' : 'rgba(239,68,68,0.3)'}`,
                  borderRadius: 'var(--radius-sm)',
                  cursor: 'pointer',
                }}
              >
                {autostart === null ? '…' : autostart ? 'Enabled' : 'Disabled'}
              </button>
            </div>
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
