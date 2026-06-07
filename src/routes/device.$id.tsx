import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
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
  const [cameraStreaming, setCameraStreaming] = useState(false)
  const [cameraIp, setCameraIp] = useState('')
  const [cameraPort, setCameraPort] = useState(0)
  const [cameraError, setCameraError] = useState<string | null>(null)
  const [previewError, setPreviewError] = useState<string | null>(null)
  const [frameSrc, setFrameSrc] = useState<string | null>(null)
  const [, setErrorCount] = useState(0)
  const [virtualCameraActive, setVirtualCameraActive] = useState(false)
  const [virtualCameraError, setVirtualCameraError] = useState<string | null>(null)

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
    
    // Listen for camera stream changes
    const unlistenCamera = listen<{ streaming: boolean; ip?: string; port?: number; use_adb?: boolean }>(
      'camera-stream-state-changed',
      (event) => {
        const payload = event.payload
        setCameraStreaming(payload.streaming)
        if (payload.streaming && payload.ip && payload.port) {
          const useAdb = payload.use_adb ?? false
          setCameraIp(useAdb ? '127.0.0.1' : payload.ip)
          setCameraPort(useAdb ? 40000 : payload.port)
          setCameraError(null)
          setPreviewError(null)
          setFrameSrc(null)
          setErrorCount(0)
          invoke('start_virtual_camera', { ip: payload.ip, port: payload.port, useAdb })
            .catch((e) => {
              console.error('Failed to start virtual camera loop:', e)
              setCameraError(typeof e === 'string' ? e : JSON.stringify(e))
            })
        } else {
          setCameraIp('')
          setCameraPort(0)
          setCameraError(null)
          setPreviewError(null)
          setFrameSrc(null)
          setErrorCount(0)
          invoke('stop_virtual_camera').catch((e) => {
            console.error('Failed to stop virtual camera loop:', e)
          })
        }
      }
    )

    // Listen for virtual camera state changes
    const unlistenVirtualCamera = listen<{ active: boolean; error: string | null }>(
      'virtual-camera-state-changed',
      (event) => {
        const { active, error } = event.payload
        setVirtualCameraActive(active)
        setVirtualCameraError(error)
      }
    )

    return () => {
      unlistenStart.then((fn) => fn()).catch(() => {})
      unlistenFinish.then((fn) => fn()).catch(() => {})
      unlistenCamera.then((fn) => fn()).catch(() => {})
      unlistenVirtualCamera.then((fn) => fn()).catch(() => {})
    }
  }, [id])

  const toggleCameraStream = async () => {
    const nextState = !cameraStreaming
    setCameraError(null)
    setPreviewError(null)
    setFrameSrc(null)
    setErrorCount(0)
    setVirtualCameraActive(false)
    setVirtualCameraError(null)
    await invoke('toggle_camera_stream', { deviceId: id, start: nextState }).catch((e) => {
      console.error('Failed to toggle camera stream:', e)
    })
  }

  useEffect(() => {
    if (!cameraStreaming || !cameraIp || !cameraPort) {
      setFrameSrc(null)
      return
    }

    // Use native MJPEG stream for both ADB and Wi-Fi.
    // The browser natively handles multipart/x-mixed-replace.
    setFrameSrc(`http://${cameraIp}:${cameraPort}/?t=${Date.now()}`)
  }, [cameraStreaming, cameraIp, cameraPort])



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
                />
                <SettingRow
                  title="Phone Camera Stream"
                  description="Use your phone camera as a virtual webcam on your PC."
                  control={<Toggle enabled={cameraStreaming} onToggle={toggleCameraStream} id="toggle-camera" />}
                  isLast={true}
                />
              </div>
            </section>

            {/* Camera Preview Section */}
            {cameraStreaming && cameraIp && cameraPort && (
              <section>
                <SectionHeader
                  title="Camera Stream Preview"
                  description={`Live view from ${peer.name}. Exposing as system video device.`}
                />
                <div style={{
                  background: 'var(--bg-surface)',
                  border: '1px solid var(--border)',
                  borderRadius: 'var(--radius-lg)',
                  overflow: 'hidden',
                  position: 'relative',
                  aspectRatio: '4/3',
                  boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
                  marginBottom: '16px'
                }}>
                  {cameraError || previewError ? (
                    <div style={{
                      position: 'absolute',
                      inset: 0,
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      justifyContent: 'center',
                      padding: '24px',
                      background: 'rgba(20, 20, 22, 0.9)',
                      color: 'var(--text-primary)',
                      textAlign: 'center'
                    }}>
                      <div style={{ color: 'var(--danger)', fontSize: '15px', fontWeight: 600, marginBottom: '8px' }}>
                        Stream Connection Error
                      </div>
                      <div style={{ fontSize: '13px', color: 'var(--text-secondary)', maxWidth: '80%' }}>
                        {cameraError || previewError}
                      </div>
                    </div>
                  ) : frameSrc ? (
                    <img
                      src={frameSrc}
                      alt="Camera stream"
                      onError={() => {
                        setErrorCount(c => {
                          const nextCount = c + 1;
                          if (nextCount < 15) {
                            // Retry by appending a new timestamp to force a reload
                            setTimeout(() => {
                              if (cameraStreaming) {
                                setFrameSrc(`http://${cameraIp}:${cameraPort}/?t=${Date.now()}`);
                              }
                            }, 500);
                          } else {
                            setPreviewError("The image preview failed to load. The loopback address may be blocked or unreachable.");
                          }
                          return nextCount;
                        });
                      }}
                      onLoad={() => setErrorCount(0)}
                      style={{
                        width: '100%',
                        height: '100%',
                        objectFit: 'cover'
                      }}
                    />
                  ) : (
                    <div style={{
                      position: 'absolute',
                      inset: 0,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      background: 'rgba(20, 20, 22, 0.5)'
                    }}>
                      <div className="progress-spinner" style={{ width: 24, height: 24 }} />
                    </div>
                  )}

                  <div style={{
                    position: 'absolute',
                    top: 12,
                    left: 12,
                    background: 'rgba(0, 0, 0, 0.65)',
                    backdropFilter: 'blur(8px)',
                    WebkitBackdropFilter: 'blur(8px)',
                    padding: '4px 10px',
                    borderRadius: '8px',
                    fontSize: '11px',
                    fontWeight: 600,
                    color: '#fff',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    border: '1px solid rgba(255, 255, 255, 0.1)'
                  }}>
                    <span style={{
                      width: 8,
                      height: 8,
                      borderRadius: '50%',
                      background: '#10b981',
                      display: 'inline-block'
                    }} />
                    LIVE ({cameraIp}:{cameraPort})
                  </div>
                </div>

                {/* Virtual Camera Status & Troubleshooting */}
                <div style={{
                  background: 'var(--bg-surface)',
                  border: '1px solid var(--border)',
                  borderRadius: 'var(--radius-lg)',
                  padding: '16px 20px',
                  boxShadow: '0 4px 20px rgba(0,0,0,0.1)',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '12px'
                }}>
                  {virtualCameraActive ? (
                    <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                      <div style={{
                        width: 28,
                        height: 28,
                        borderRadius: '50%',
                        background: 'rgba(16, 185, 129, 0.15)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: '#10b981',
                        flexShrink: 0
                      }}>
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                          <polyline points="20 6 9 17 4 12"></polyline>
                        </svg>
                      </div>
                      <div>
                        <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
                          System Virtual Camera Active
                        </div>
                        <div style={{ fontSize: '12.5px', color: 'var(--text-tertiary)', marginTop: '2px' }}>
                          Available in Zoom, Meet, and other platforms as <code style={{ background: 'var(--bg-elevated)', padding: '2px 6px', borderRadius: '4px', fontSize: '11px', color: 'var(--accent)' }}>Sync Camera</code> (/dev/video9).
                        </div>
                      </div>
                    </div>
                  ) : virtualCameraError ? (
                    <div>
                      <div style={{ display: 'flex', alignItems: 'start', gap: '12px', marginBottom: '12px' }}>
                        <div style={{
                          width: 28,
                          height: 28,
                          borderRadius: '50%',
                          background: 'rgba(239, 68, 68, 0.15)',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          color: 'var(--danger)',
                          flexShrink: 0,
                          marginTop: '2px'
                        }}>
                          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                            <line x1="18" y1="6" x2="6" y2="18"></line>
                            <line x1="6" y1="6" x2="18" y2="18"></line>
                          </svg>
                        </div>
                        <div style={{ flex: 1 }}>
                          <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
                            Virtual Camera Driver Error
                          </div>
                          <div style={{ fontSize: '12.5px', color: 'var(--danger)', marginTop: '2px', wordBreak: 'break-word' }}>
                            {virtualCameraError}
                          </div>
                        </div>
                      </div>
                      
                      <div style={{
                        background: 'var(--bg-elevated)',
                        border: '1px solid var(--border)',
                        borderRadius: 'var(--radius-md)',
                        padding: '12px 16px',
                        fontSize: '12.5px'
                      }}>
                        <div style={{ fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '8px' }}>
                          To make this camera available in Zoom and other apps:
                        </div>
                        <ol style={{ paddingLeft: '20px', margin: '0 0 12px 0', color: 'var(--text-tertiary)', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                          <li>Make sure `v4l2loopback` is installed: <code style={{ color: 'var(--text-primary)' }}>sudo apt install v4l2loopback-dkms v4l2loopback-utils</code></li>
                          <li>Run this command in your terminal to load the driver:
                            <div style={{
                              display: 'flex',
                              alignItems: 'center',
                              background: 'var(--bg-base)',
                              padding: '6px 10px',
                              borderRadius: '4px',
                              marginTop: '4px',
                              fontFamily: 'monospace',
                              fontSize: '11px',
                              color: 'var(--accent)',
                              border: '1px solid var(--border)',
                              overflowX: 'auto',
                              whiteSpace: 'pre'
                            }}>
                              sudo modprobe v4l2loopback exclusive_caps=1 card_label="Sync Camera" video_nr=9
                            </div>
                          </li>
                          <li>If you see a permission error, ensure you have write access:
                            <div style={{
                              display: 'flex',
                              alignItems: 'center',
                              background: 'var(--bg-base)',
                              padding: '6px 10px',
                              borderRadius: '4px',
                              marginTop: '4px',
                              fontFamily: 'monospace',
                              fontSize: '11px',
                              color: 'var(--accent)',
                              border: '1px solid var(--border)',
                              overflowX: 'auto',
                              whiteSpace: 'pre'
                            }}>
                              sudo chmod 0666 /dev/video9
                            </div>
                          </li>
                        </ol>
                        <div style={{ color: 'var(--text-tertiary)', fontSize: '11.5px', fontStyle: 'italic' }}>
                          Note: Once you run these commands, turn this stream Off and back On.
                        </div>
                      </div>
                    </div>
                  ) : (
                    <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                      <div style={{ width: 16, height: 16, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}>
                        <div className="progress-spinner" style={{ width: 14, height: 14 }} />
                      </div>
                      <div>
                        <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
                          Starting virtual camera driver...
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </section>
            )}


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