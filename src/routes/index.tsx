import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Wifi, Box } from 'lucide-react'
import { usePeers } from '../hooks/use-peers'
import { getBrand } from '../utils/device'
import { DeviceCard } from '../components/ui/device-card'
import { SettingsModal } from '../components/layout/settings-modal'
import { PairModal } from '../components/layout/pair-modal'

export const Route = createFileRoute('/')({
  component: RouteComponent,
})

function RouteComponent() {
  const navigate = useNavigate({ from: Route.fullPath })
  const [showSettings, setShowSettings] = useState(false)
  const {
    peers,
    pending,
    broadcasting,
    refreshPeers,
    resolvePairRequest,
    toggleBroadcasting,
  } = usePeers()

  const currentRequest = pending[0]

  return (
    <div className="app-shell">
      {/* ─── Top Bar ─────────────────────── */}
      <header className="top-bar">
        <div className="top-bar-left">
          <div className="top-bar-logo">
            <Box size={20} strokeWidth={2.5} style={{ marginRight: 4 }} />
            <span className="top-bar-logo-text">Sync</span>
          </div>
        </div>

        <div className="top-bar-right">
          {/* iOS-Style Toggle Switch */}
          <button
            onClick={toggleBroadcasting}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              padding: '4px',
              borderRadius: '999px',
              background: 'transparent',
              border: 'none',
              cursor: 'pointer',
              marginRight: 8
            }}
            title={broadcasting ? 'Visible: Click to hide' : 'Hidden: Click to broadcast'}
          >
            <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>Visible</span>
            <div style={{
              width: 36,
              height: 20,
              borderRadius: 10,
              background: broadcasting ? 'var(--success)' : 'var(--bg-elevated)',
              border: '1px solid var(--border)',
              position: 'relative',
              transition: 'background 0.2s',
            }}>
              <div style={{
                position: 'absolute',
                top: 1,
                left: broadcasting ? 17 : 1,
                width: 16,
                height: 16,
                borderRadius: '50%',
                background: '#fff',
                transition: 'left 0.2s cubic-bezier(0.2, 0.8, 0.2, 1)',
                boxShadow: '0 1px 3px rgba(0,0,0,0.3)',
              }} />
            </div>
          </button>

          <button className="btn btn-ghost btn-icon" style={{ border: 'none' }} onClick={refreshPeers} title="Refresh peers">
            <Wifi size={14} strokeWidth={2} />
          </button>

          <button className="btn btn-ghost btn-icon" style={{ border: 'none' }} onClick={() => setShowSettings(true)} title="Settings">
            <Box size={16} style={{ display: 'none' }} /> {/* dummy just to keep layout if needed, or simply render standard layout */}
            <span style={{ display: 'flex', alignItems: 'center' }}>⚙️</span>
          </button>
        </div>
      </header>

      {/* ─── Main Content ─────────────────── */}
      <main className="main-content">
        <div className="dashboard-wrap">
          {/* Header */}
          <div className="page-header" style={{ marginBottom: 12 }}>
            <div>
              <div className="page-title">Paired Devices</div>
            </div>
          </div>

          {/* Device list */}
          <div className="section">
            {peers.length === 0 ? (
              <div className="empty-state">
                <div className="empty-state-icon">📱</div>
                <div className="empty-state-text">No paired devices yet</div>
                <div className="empty-state-sub">
                  Open the Sync app on your Android and scan for this desktop
                </div>
              </div>
            ) : (
              <div className="device-list">
                {peers.map((peer) => (
                  <DeviceCard
                    key={peer.device_id}
                    peer={peer}
                    onClick={(p) => navigate({ to: '/device/$id', params: { id: p.device_id } })}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Pending connection requests */}
          {pending.length > 1 && (
            <>
              <div className="section" style={{ marginTop: 24 }}>
                <div className="section-header">
                  <span className="section-title">Pending Connections</span>
                </div>
                <div className="device-list">
                  {pending.slice(1).map((req) => {
                    const logo = getBrand(req.name);
                    return (
                      <div key={req.deviceId} className="device-card">
                        <div className="device-card-header">
                          <div className="device-icon-wrap" style={{ height: 28 }}>
                            {logo}
                          </div>
                        </div>
                        <div className="device-info">
                          <div className="device-name">{req.name}</div>
                          <div className="device-meta">Waiting for verification…</div>
                        </div>
                        <div className="device-card-footer">
                          <div className="badge badge-connecting" style={{ background: 'transparent', border: 'none', padding: 0 }}>
                            <span style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--warning)', display: 'inline-block' }} />
                            <span style={{ color: 'var(--text-secondary)' }}>Pending</span>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            </>
          )}

        </div>
      </main>

      {/* ─── Settings Modal ───────────────── */}
      {showSettings && (
        <SettingsModal onClose={() => setShowSettings(false)} />
      )}

      {/* ─── Pair Request Modal ───────────── */}
      {currentRequest && (
        <PairModal request={currentRequest} onResolve={resolvePairRequest} />
      )}
    </div>
  )
}
