import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { ChevronLeft, Server, Clock } from 'lucide-react'
import { Toggle } from '../components/ui/toggle'
import { SectionHeader } from '../components/ui/section-header'
import { SettingRow } from '../components/ui/setting-row'
import { useDeviceSettings } from '../hooks/use-device-settings'
import { getBrand, relativeTime } from '../utils/device'

export const Route = createFileRoute('/device/$id')({
  component: DeviceRoute,
})

function DeviceRoute() {
  const { id } = Route.useParams()
  const navigate = useNavigate({ from: Route.fullPath })
  const [confirming, setConfirming] = useState(false)
  const {
    peer,
    unpairPeer,
    toggleClipboardSync,
    toggleMediaControls,
    toggleVolumeSync,
    toggleIncomingFiles,
    toggleTerminalAccess,
  } = useDeviceSettings(id)

  const [transferring, setTransferring] = useState(false)

  useEffect(() => {
    const unlistenStart = listen<string>('file-transfer-started', (event) => {
      if (event.payload === id) {
        setTransferring(true)
      }
    })
    const unlistenFinish = listen<string>('file-transfer-finished', (event) => {
      if (event.payload === id) {
        setTransferring(false)
      }
    })
    return () => {
      unlistenStart.then((fn) => fn()).catch(() => {})
      unlistenFinish.then((fn) => fn()).catch(() => {})
    }
  }, [id])

  if (!peer) {
    return (
      <div className="app-shell" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <div className="status-dot" style={{ width: 12, height: 12 }} />
      </div>
    )
  }

  const logo = getBrand(peer.name)

  return (
    <div className="app-shell">
      {/* Top Bar with glassmorphism */}
      <header className="top-bar" style={{
        position: 'sticky',
        top: 0,
        zIndex: 10,
        background: 'rgba(18, 18, 20, 0.75)',
        backdropFilter: 'blur(16px)',
        WebkitBackdropFilter: 'blur(16px)',
      }}>
        <div className="top-bar-left">
          <button
            className="btn btn-ghost btn-icon"
            onClick={() => navigate({ to: '/' })}
            style={{
              marginRight: 12,
              border: 'none',
              background: 'rgba(255, 255, 255, 0.05)',
            }}
          >
            <ChevronLeft size={20} strokeWidth={2.5} />
          </button>
          <div className="page-title" style={{ margin: 0, fontSize: 18 }}>Device Settings</div>
        </div>
      </header>

      {/* Scrollable Content */}
      <div className="main-content scrollable">
        <div className="dashboard-wrap" style={{ maxWidth: 760, padding: '24px 20px', margin: '0 auto' }}>

          {/* Header Card */}
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: '20px',
            marginBottom: '32px',
            padding: '24px',
            background: 'linear-gradient(145deg, var(--bg-surface), var(--bg-base))',
            borderRadius: 'var(--radius-xl)',
            border: '1px solid var(--border)',
            boxShadow: '0 8px 32px rgba(0,0,0,0.15)'
          }}>
            <div style={{
              width: 64, height: 64,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'var(--bg-elevated)',
              borderRadius: '16px',
              border: '1px solid var(--border)',
              boxShadow: 'inset 0 2px 10px rgba(255,255,255,0.02), 0 4px 12px rgba(0,0,0,0.2)'
            }}>
              {transferring ? (
                <div style={{ width: 32, height: 32 }}>
                  <div className="progress-spinner" />
                </div>
              ) : (
                <div style={{ transform: 'scale(1.3)', color: 'var(--accent)' }}>{logo}</div>
              )}
            </div>

            <div style={{ flex: 1 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '6px' }}>
                <h2 style={{ fontSize: '24px', fontWeight: 700, margin: 0, color: 'var(--text-primary)', letterSpacing: '-0.5px' }}>
                  {peer.name}
                </h2>
                <span className={`badge ${peer.connected ? 'badge-connected' : 'badge-offline'}`} style={{ padding: '4px 10px', fontSize: '11px' }}>
                  {peer.connected ? 'Connected' : 'Offline'}
                </span>
              </div>
              <div style={{ display: 'flex', gap: '16px', color: 'var(--text-tertiary)', fontSize: '12.5px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <Server size={14} strokeWidth={2} />
                  ID: {peer.device_id.split('-')[0]}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <Clock size={14} strokeWidth={2} />
                  Seen {relativeTime(peer.last_seen)}
                </div>
              </div>
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>

            {/* Features Section */}
            <section>
              <SectionHeader
                title="Integrations"
                description="Manage what features are shared between your PC and this device."
              />
              <div style={{
                background: 'var(--bg-surface)',
                border: '1px solid var(--border)',
                borderRadius: 'var(--radius-lg)',
                overflow: 'hidden',
                boxShadow: '0 4px 20px rgba(0,0,0,0.1)'
              }}>
                <SettingRow
                  title="Universal Clipboard"
                  description="Automatically synchronize clipboard text and images across your devices."
                  control={<Toggle enabled={peer.clipboard_sync_enabled} onToggle={toggleClipboardSync} id="toggle-clipboard" />}
                />
                <SettingRow
                  title="Media Controls"
                  description="Allow this device to view and control currently playing media on your PC."
                  control={<Toggle enabled={peer.media_controls_enabled} onToggle={toggleMediaControls} id="toggle-media" />}
                />
                <SettingRow
                  title="Volume Synchronization"
                  description="Sync the master volume level of your PC with this device."
                  control={<Toggle enabled={peer.volume_sync_enabled} onToggle={toggleVolumeSync} id="toggle-volume" />}
                />
                <SettingRow
                  title="Receive Files"
                  description="Allow this device to send files directly to your PC's Downloads folder."
                  control={<Toggle enabled={peer.incoming_files_enabled} onToggle={toggleIncomingFiles} id="toggle-files" />}
                />
                <SettingRow
                  title="Terminal Access"
                  description="Allow this device to securely access the command line terminal on your PC."
                  control={<Toggle enabled={peer.terminal_access_enabled} onToggle={toggleTerminalAccess} id="toggle-terminal" />}
                  isLast={true}
                />
              </div>
            </section>

            {/* Danger Zone */}
            <section>
              <SectionHeader
                title="Danger Zone"
                description="Irreversible actions for this device connection."
              />
              <div style={{
                background: 'rgba(248, 113, 113, 0.05)',
                border: '1px solid rgba(248, 113, 113, 0.15)',
                borderRadius: 'var(--radius-lg)',
                padding: '16px 20px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between'
              }}>
                <div>
                  <div style={{ fontSize: '14.5px', fontWeight: 600, color: 'var(--danger)', marginBottom: '6px' }}>Unpair Device</div>
                  <div style={{ fontSize: '13px', color: 'var(--text-tertiary)' }}>Revoke access and remove this device from your trusted list permanently.</div>
                </div>
                <div style={{ flexShrink: 0, marginLeft: '24px' }}>
                  {confirming ? (
                    <div style={{ display: 'flex', gap: '8px' }}>
                      <button className="btn btn-ghost" onClick={() => setConfirming(false)} style={{ background: 'var(--bg-surface)' }}>Cancel</button>
                      <button className="btn btn-danger" onClick={unpairPeer}>Confirm</button>
                    </div>
                  ) : (
                    <button className="btn btn-danger" onClick={() => setConfirming(true)}>Unpair</button>
                  )}
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  )
}