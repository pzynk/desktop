import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { Wifi, Settings, Rocket, Smartphone } from 'lucide-react'
import { usePeers } from '../hooks/use-peers'
import { getBrand } from '../utils/device'
import { DeviceCard } from '../components/ui/device-card'
import { SettingsModal } from '../components/layout/settings-modal'
import { PairModal } from '../components/layout/pair-modal'
import { check } from '@tauri-apps/plugin-updater'
import { invoke } from '@tauri-apps/api/core'
import logo from '../assets/logo.png'

export const Route = createFileRoute('/')({
  component: RouteComponent,
})

function RouteComponent() {
  const navigate = useNavigate({ from: Route.fullPath })
  const [showSettings, setShowSettings] = useState(false)
  const [updateAvailable, setUpdateAvailable] = useState<any>(null)
  const [updateStatus, setUpdateStatus] = useState<'idle' | 'checking' | 'updating' | 'installing' | 'error'>('idle')
  const [downloadProgress, setDownloadProgress] = useState(0)

  useEffect(() => {
    const checkUpdates = async () => {
      try {
        setUpdateStatus('checking')
        const update = await check()
        if (update && update.available) {
          setUpdateAvailable(update)
        }
        setUpdateStatus('idle')
      } catch (err) {
        console.error('Failed to check for updates:', err)
        setUpdateStatus('idle')
      }
    }
    checkUpdates()
  }, [])

  const handleUpdate = async () => {
    if (!updateAvailable) return
    try {
      setUpdateStatus('updating')
      setDownloadProgress(0)

      let total = 0
      let downloaded = 0

      await updateAvailable.downloadAndInstall((p: any) => {
        if (p.event === 'Started') {
          total = p.data.contentLength || 0
        } else if (p.event === 'Progress') {
          let length = p.data.chunkLength || 0
          downloaded += length
          if (total > 0) {
            let percentage = Math.round((downloaded / total) * 100)
            setDownloadProgress(percentage)
          }
        } else if (p.event === 'Finished') {
          setDownloadProgress(100)
          setUpdateStatus('installing')
        }
      })

      try {
        await invoke('relaunch_app')
      } catch (error: any) {
        console.error('Failed to relaunch:', error?.message || 'Unknown error')
        throw error
      }
    } catch (error: any) {
      console.error('Update failed:', error?.message || 'Unknown error')
      setUpdateStatus('error')
      alert(`Update failed: ${error?.message || 'Unknown error'}`)
    }
  }

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
            <img src={logo} alt="Pzync Logo" style={{ width: 20, height: 20, marginRight: 6, objectFit: 'contain' }} />
            <span className="top-bar-logo-text">Pzync</span>
          </div>
        </div>

        <div className="top-bar-right">
          {/* Update UI has been moved to a banner below the header */}

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
            <Settings size={14} strokeWidth={2} />
          </button>
        </div>
      </header>

      {/* ─── Main Content ─────────────────── */}
      <main className="main-content">
        <div className="dashboard-wrap">
          {/* ─── Update Banner ─────────────────── */}
          {(updateAvailable || updateStatus === 'updating') && (
            <div style={{
              background: 'linear-gradient(135deg, rgba(6,182,212,0.08) 0%, rgba(180,142,247,0.04) 100%)',
              border: '1px solid rgba(6,182,212,0.2)',
              borderRadius: 'var(--radius-lg)',
              padding: '16px 20px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              boxShadow: '0 8px 32px rgba(0,0,0,0.1)',
              position: 'relative',
              overflow: 'hidden'
            }}>
              <div style={{ position: 'absolute', top: -40, right: -40, width: 100, height: 100, background: 'var(--accent)', filter: 'blur(50px)', opacity: 0.15, pointerEvents: 'none' }} />

              <div>
                <div style={{ fontSize: 15, fontWeight: 600, color: '#fff', marginBottom: 4, display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Rocket size={16} /> Update Available
                </div>
                <div style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                  Version {updateAvailable?.version || ''} is ready to install.
                </div>
              </div>

              {updateStatus === 'updating' || updateStatus === 'installing' ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: 'var(--accent)', background: 'rgba(6,182,212,0.1)', padding: '6px 14px', borderRadius: 'var(--radius-sm)', border: '1px solid rgba(6,182,212,0.2)' }}>
                    <span className="progress-spinner" style={{ width: 14, height: 14, borderWidth: 2 }} />
                    {updateStatus === 'updating' ? `Downloading... ${downloadProgress}%` : 'Installing...'}
                  </div>
                  {updateStatus === 'updating' && (
                    <div style={{ width: '100%', height: 4, background: 'rgba(255,255,255,0.1)', borderRadius: 2, overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${downloadProgress}%`, background: 'var(--accent)', transition: 'width 0.2s' }} />
                    </div>
                  )}
                </div>
              ) : (
                <button
                  onClick={handleUpdate}
                  className="btn btn-primary"
                  style={{ padding: '8px 16px', background: 'var(--accent)', color: 'var(--bg-base)', border: 'none' }}
                >
                  Install Update
                </button>
              )}
            </div>
          )}

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
                <div className="empty-state-icon"><Smartphone size={36} strokeWidth={1.5} /></div>
                <div className="empty-state-text">No paired devices yet</div>
                <div className="empty-state-sub">
                  Open the Pzync app on your Android and scan for this desktop
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
