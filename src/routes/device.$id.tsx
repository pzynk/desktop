import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { ChevronLeft, ChevronRight, Server, Clock, Mic, MicOff, Volume2, Camera, Video, Sliders } from 'lucide-react'
import { Toggle } from '../components/ui/toggle'
import { SectionHeader } from '../components/ui/section-header'
import { SettingRow } from '../components/ui/setting-row'
import { useDeviceSettings } from '../hooks/use-device-settings'
import { getBrand, relativeTime } from '../utils/device'
import { DependencyModal, MissingDependency } from '../components/layout/dependency-modal'

export const Route = createFileRoute('/device/$id')({
  component: DeviceRoute,
})

function DeviceRoute() {
  const { id } = Route.useParams()
  const navigate = useNavigate({ from: Route.fullPath })
  const [confirming, setConfirming] = useState(false)
  const [missingDependency, setMissingDependency] = useState<MissingDependency | null>(null)
  const {
    peer,
    unpairPeer,
    toggleClipboardSync,
    toggleMediaControls,
    toggleVolumeSync,
    toggleIncomingFiles,
    toggleTerminalAccess,
    toggleAudioStreaming,
  } = useDeviceSettings(id)

  const [transferring, setTransferring] = useState(false)
  const [cameraStreaming, setCameraStreaming] = useState(false)

  const [micStreaming, setMicStreaming] = useState(false)
  const [_virtualMicActive, setVirtualMicActive] = useState(false)
  const [_virtualMicError, setVirtualMicError] = useState<string | null>(null)
  const [micAudioLevel, setMicAudioLevel] = useState(0)
  const [micMuted, setMicMuted] = useState(false)
  const [micVolume, setMicVolume] = useState(1.0)

  useEffect(() => {
    invoke('request_camera_config', { deviceId: id }).catch(() => {})
    invoke<boolean>('get_camera_stream_state').then((active) => setCameraStreaming(active)).catch(() => {})
    invoke<boolean>('get_mic_stream_state').then((active) => setMicStreaming(active)).catch(() => {})

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
    const unlistenCamera = listen<{ streaming: boolean }>(
      'camera-stream-state-changed',
      (event) => {
        setCameraStreaming(event.payload.streaming)
      }
    )

    const unlistenMic = listen<{ streaming: boolean; ip?: string; port?: number; sample_rate?: number; channels?: number; use_adb?: boolean }>(
      'mic-stream-state-changed',
      (event) => {
        const payload = event.payload
        setMicStreaming(payload.streaming)
      }
    )

    const unlistenVirtualMic = listen<{ active: boolean; error: string | null }>(
      'virtual-mic-state-changed',
      (event) => {
        setVirtualMicActive(event.payload.active)
        setVirtualMicError(event.payload.error)
      }
    )

    const unlistenAudioLevel = listen<number>('mic-audio-level', (event) => {
      setMicAudioLevel(event.payload)
    })

    return () => {
      unlistenStart.then((fn) => fn()).catch(() => {})
      unlistenFinish.then((fn) => fn()).catch(() => {})
      unlistenCamera.then((fn) => fn()).catch(() => {})
      unlistenMic.then((fn) => fn()).catch(() => {})
      unlistenVirtualMic.then((fn) => fn()).catch(() => {})
      unlistenAudioLevel.then((fn) => fn()).catch(() => {})
    }
  }, [id])

  const toggleMicStream = async () => {
    const next = !micStreaming
    setVirtualMicError(null)
    if (next) {
      const missing = await invoke<MissingDependency | null>('check_system_deps', { feature: 'mic' }).catch(() => null)
      if (missing) {
        setMissingDependency(missing)
        return
      }
    } else {
      setMicStreaming(false)
      setMicAudioLevel(0)
    }
    await invoke('toggle_mic_stream', { deviceId: id, start: next }).catch((e) => {
      console.error('Failed to toggle mic stream:', e)
      if (next) {
        setMicStreaming(false)
      }
    })
  }

  const toggleMicMute = async () => {
    const next = !micMuted
    setMicMuted(next)
    await invoke('set_virtual_mic_muted', { muted: next }).catch((e) => console.error(e))
  }

  const handleMicVolumeChange = async (v: number) => {
    setMicVolume(v)
    await invoke('set_virtual_mic_volume', { volume: v }).catch((e) => console.error(e))
  }

  const toggleCameraStream = async () => {
    const nextState = !cameraStreaming
    if (nextState) {
      const missing = await invoke<MissingDependency | null>('check_system_deps', { feature: 'camera' }).catch(() => null)
      if (missing) {
        setMissingDependency(missing)
        return
      }
    } else {
      setCameraStreaming(false)
    }
    await invoke('toggle_camera_stream', { deviceId: id, start: nextState }).catch((e) => {
      console.error('Failed to toggle camera stream:', e)
      if (nextState) {
        setCameraStreaming(false)
      }
    })
  }



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
                  title="Listen Through Mobile"
                  description="Stream your PC's system audio to this device so you can listen through its speaker or headphones."
                  control={<Toggle enabled={peer.audio_streaming_enabled} onToggle={toggleAudioStreaming} id="toggle-audio" />}
                  isLast={true}
                />
              </div>
            </section>

            {/* System Virtual Camera */}
            <section>
              <SectionHeader
                title="Virtual Camera"
                description="Use your phone as a high-definition virtual webcam in Google Meet, Zoom, Teams, and OBS."
              />
              <div style={{
                background: 'var(--bg-surface)',
                border: '1px solid var(--border)',
                borderRadius: 'var(--radius-lg)',
                padding: '20px',
                display: 'flex',
                flexDirection: 'column',
                gap: '14px',
                boxShadow: '0 4px 20px rgba(0,0,0,0.06)'
              }}>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                    <div style={{
                      width: 40,
                      height: 40,
                      borderRadius: 'var(--radius-md)',
                      background: cameraStreaming ? 'rgba(59, 130, 246, 0.15)' : 'var(--bg-elevated)',
                      color: cameraStreaming ? 'var(--accent)' : 'var(--text-tertiary)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      transition: 'all 0.2s ease'
                    }}>
                      {cameraStreaming ? <Video width={20} height={20} /> : <Camera width={20} height={20} />}
                    </div>
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-primary)' }}>
                          Phone Camera
                        </span>
                        {cameraStreaming && (
                          <span style={{
                            fontSize: '10px',
                            fontWeight: 700,
                            letterSpacing: '0.04em',
                            padding: '1px 6px',
                            borderRadius: '4px',
                            background: 'rgba(16, 185, 129, 0.15)',
                            color: '#10b981',
                            border: '1px solid rgba(16, 185, 129, 0.3)'
                          }}>
                            LIVE
                          </span>
                        )}
                      </div>
                      <div style={{ fontSize: '12.5px', color: 'var(--text-tertiary)' }}>
                        {cameraStreaming
                          ? 'Streaming active to system virtual camera device'
                          : 'Turn on to stream high-definition video as virtual webcam'}
                      </div>
                    </div>
                  </div>
                  <Toggle enabled={cameraStreaming} onToggle={toggleCameraStream} id="toggle-camera" />
                </div>

                {cameraStreaming && (
                  <div style={{
                    background: 'rgba(16, 185, 129, 0.1)',
                    border: '1px solid rgba(16, 185, 129, 0.2)',
                    borderRadius: 'var(--radius-md)',
                    padding: '9px 12px',
                    fontSize: '12.5px',
                    color: 'var(--text-secondary)',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '10px'
                  }}>
                    <div style={{ width: 8, height: 8, borderRadius: '50%', background: '#10b981', flexShrink: 0 }} />
                    <span>Appears in Zoom, Google Meet, Teams as <strong style={{ color: '#10b981' }}>Sync Camera</strong></span>
                  </div>
                )}

                <div style={{
                  paddingTop: '6px',
                  borderTop: '1px solid var(--border)'
                }}>
                  <button
                    onClick={() => navigate({ to: '/device/$id/camera', params: { id } })}
                    className="btn btn-ghost"
                    style={{
                      width: '100%',
                      padding: '10px 14px',
                      borderRadius: 'var(--radius-md)',
                      background: 'var(--bg-elevated)',
                      border: '1px solid var(--border)',
                      color: 'var(--text-primary)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      fontSize: '13px',
                      fontWeight: 500,
                      cursor: 'pointer',
                      transition: 'all 0.15s ease'
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-secondary)' }}>
                      <Sliders size={15} color="var(--accent)" />
                      <span style={{ color: 'var(--text-primary)', fontWeight: 550 }}>Camera Options & Preview</span>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-tertiary)', fontSize: '12px' }}>
                      <span>Configure</span>
                      <ChevronRight size={14} />
                    </div>
                  </button>
                </div>
              </div>
            </section>

            {/* System Virtual Microphone */}
            <section>
              <SectionHeader
                title="Virtual Microphone"
                description="Use your phone's microphone as a system input device for Google Meet, Zoom, Teams, and Discord."
              />
              <div style={{
                background: 'var(--bg-surface)',
                border: '1px solid var(--border)',
                borderRadius: 'var(--radius-lg)',
                padding: '20px',
                display: 'flex',
                flexDirection: 'column',
                gap: '16px'
              }}>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                    <div style={{
                      width: 40,
                      height: 40,
                      borderRadius: 'var(--radius-md)',
                      background: micStreaming ? 'rgba(59, 130, 246, 0.15)' : 'var(--bg-elevated)',
                      color: micStreaming ? 'var(--accent)' : 'var(--text-tertiary)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center'
                    }}>
                      {micStreaming ? <Mic width={20} height={20} /> : <MicOff width={20} height={20} />}
                    </div>
                    <div>
                      <div style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-primary)' }}>
                        Phone Microphone
                      </div>
                      <div style={{ fontSize: '12.5px', color: 'var(--text-tertiary)' }}>
                        {micStreaming ? 'Streaming active to system virtual input device' : 'Turn on to stream audio from phone mic'}
                      </div>
                    </div>
                  </div>
                  <Toggle enabled={micStreaming} onToggle={toggleMicStream} id="toggle-mic" />
                </div>

                {micStreaming && (
                  <div style={{
                    background: 'var(--bg-base)',
                    borderRadius: 'var(--radius-md)',
                    padding: '16px',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '14px',
                    border: '1px solid var(--border)'
                  }}>
                    {/* Live Level Meter */}
                    <div>
                      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px', fontSize: '12px', color: 'var(--text-secondary)' }}>
                        <span>Audio Input Level</span>
                        <span>{Math.round(micAudioLevel * 100)}%</span>
                      </div>
                      <div style={{
                        height: '8px',
                        background: 'var(--bg-elevated)',
                        borderRadius: '4px',
                        overflow: 'hidden'
                      }}>
                        <div style={{
                          height: '100%',
                          width: `${Math.min(100, Math.max(0, micAudioLevel * 100))}%`,
                          background: micAudioLevel > 0.85 ? 'var(--danger)' : 'var(--accent)',
                          transition: 'width 60ms ease-out'
                        }} />
                      </div>
                    </div>

                    {/* Mute & Gain controls */}
                    <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                      <button
                        className={`btn ${micMuted ? 'btn-danger' : 'btn-ghost'}`}
                        onClick={toggleMicMute}
                        style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '8px 14px' }}
                      >
                        {micMuted ? <MicOff width={16} height={16} /> : <Mic width={16} height={16} />}
                        <span>{micMuted ? 'Muted' : 'Mute'}</span>
                      </button>

                      <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: '10px' }}>
                        <Volume2 width={16} height={16} style={{ color: 'var(--text-tertiary)' }} />
                        <input
                          type="range"
                          min="0"
                          max="2"
                          step="0.05"
                          value={micVolume}
                          onChange={(e) => handleMicVolumeChange(parseFloat(e.target.value))}
                          style={{ flex: 1, accentColor: 'var(--accent)' }}
                        />
                        <span style={{ fontSize: '12px', color: 'var(--text-tertiary)', width: '36px' }}>
                          {Math.round(micVolume * 100)}%
                        </span>
                      </div>
                    </div>

                    {/* Device Badge */}
                    <div style={{
                      background: 'rgba(16, 185, 129, 0.1)',
                      border: '1px solid rgba(16, 185, 129, 0.2)',
                      borderRadius: 'var(--radius-md)',
                      padding: '10px 14px',
                      fontSize: '12.5px',
                      color: 'var(--text-secondary)',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '10px'
                    }}>
                      <div style={{ width: 8, height: 8, borderRadius: '50%', background: '#10b981' }} />
                      <span>Appears in System Sound Settings, Google Meet, Zoom as <strong style={{ color: '#10b981' }}>Sync Microphone</strong></span>
                    </div>
                  </div>
                )}
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
      {missingDependency && (
        <DependencyModal
          dependency={missingDependency}
          onClose={() => setMissingDependency(null)}
          onSuccess={() => {
            if (missingDependency.feature === 'mic') {
              invoke('toggle_mic_stream', { deviceId: id, start: true }).catch((e) => console.error(e))
            } else if (missingDependency.feature === 'camera') {
              invoke('toggle_camera_stream', { deviceId: id, start: true }).catch((e) => console.error(e))
            }
          }}
          onStreamDirect={() => {
            invoke('toggle_mic_stream', { deviceId: id, start: true }).catch((e) => console.error(e))
          }}
        />
      )}
    </div>
  )
}