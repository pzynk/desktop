import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { Settings } from 'lucide-react'
import { TrustedPeer, getBrand, relativeTime } from '../../utils/device'

interface DeviceCardProps {
  peer: TrustedPeer
  onClick: (peer: TrustedPeer) => void
}

export function DeviceCard({ peer, onClick }: DeviceCardProps) {
  const [transferring, setTransferring] = useState(false)

  useEffect(() => {
    const unlistenStart = listen<string>('file-transfer-started', (event) => {
      if (event.payload === peer.device_id) {
        setTransferring(true)
      }
    })
    const unlistenFinish = listen<string>('file-transfer-finished', (event) => {
      if (event.payload === peer.device_id) {
        setTransferring(false)
      }
    })
    return () => {
      unlistenStart.then((fn) => fn()).catch(() => {})
      unlistenFinish.then((fn) => fn()).catch(() => {})
    }
  }, [peer.device_id])

  const logo = getBrand(peer.name)

  return (
    <div className="device-card" onClick={() => onClick(peer)} style={{ cursor: 'pointer' }}>
      <div className="device-card-header">
        <div className="device-icon-wrap">
          {transferring ? <div className="progress-spinner" /> : logo}
        </div>
      </div>

      <div className="device-info">
        <div className="device-name">{peer.name}</div>
        <div className="device-meta">Last seen {relativeTime(peer.last_seen)}</div>
      </div>

      <div className="device-card-footer">
        <div className="badge" style={{ background: 'transparent', border: 'none', padding: 0, display: 'inline-flex', alignItems: 'center', gap: 6 }}>
          <span style={{
            width: 6,
            height: 6,
            borderRadius: '50%',
            background: peer.connected ? 'var(--success)' : 'var(--text-tertiary)',
            display: 'inline-block'
          }} />
          <span style={{ color: 'var(--text-secondary)' }}>
            {peer.connected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        <div style={{ color: 'var(--text-tertiary)' }}>
          <Settings size={16} strokeWidth={2} />
        </div>
      </div>
    </div>
  )
}
