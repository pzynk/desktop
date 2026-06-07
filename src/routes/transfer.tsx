import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { ChevronLeft, FileUp, CheckCircle, WifiOff } from 'lucide-react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

export const Route = createFileRoute('/transfer')({
  component: TransferComponent,
})

type TransferProgress = {
  device_id: string
  filename: string
  bytes_received: number
  total_bytes: number
  speed_bytes_per_sec: number
  status?: 'Receiving' | 'Decoding' | 'Verifying' | 'Saving'
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function formatSpeed(bytesPerSec: number): string {
  return formatBytes(bytesPerSec) + '/s'
}

function TransferComponent() {
  const navigate = useNavigate({ from: Route.fullPath })
  const [transfer, setTransfer] = useState<TransferProgress | null>(null)
  const [status, setStatus] = useState<'receiving' | 'completed' | 'idle'>('idle')

  useEffect(() => {
    // Check initial active transfer state
    invoke<TransferProgress | null>('get_active_transfer')
      .then((data) => {
        if (data) {
          setTransfer(data)
          setStatus('receiving')
        }
      })
      .catch(() => {})

    const unlistenStart = listen<string>('file-transfer-started', () => {
      setStatus('receiving')
    })

    const unlistenProgress = listen<TransferProgress>('file-transfer-progress', (event) => {
      setTransfer(event.payload)
      setStatus('receiving')
    })

    const unlistenFinish = listen<string>('file-transfer-finished', () => {
      setStatus('completed')
    })

    return () => {
      unlistenStart.then((fn) => fn()).catch(() => {})
      unlistenProgress.then((fn) => fn()).catch(() => {})
      unlistenFinish.then((fn) => fn()).catch(() => {})
    }
  }, [])

  const isValidating = transfer && transfer.status && transfer.status !== 'Receiving'

  const percent = isValidating
    ? 100
    : (transfer && transfer.total_bytes > 0
      ? Math.min(Math.round((transfer.bytes_received / transfer.total_bytes) * 100), 100)
      : 0)

  const pendingBytes = transfer ? Math.max(transfer.total_bytes - transfer.bytes_received, 0) : 0

  // Calculate ETA (seconds remaining)
  const eta = transfer && transfer.speed_bytes_per_sec > 0
    ? Math.round(pendingBytes / transfer.speed_bytes_per_sec)
    : null

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
              border: 'none',
              background: 'rgba(255, 255, 255, 0.05)',
            }}
          >
            <ChevronLeft size={20} strokeWidth={2.5} />
          </button>
          <div className="page-title" style={{ margin: 0, fontSize: 18 }}>File Transfer</div>
        </div>
      </header>

      {/* Scrollable Content */}
      <div className="main-content scrollable" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <div style={{
          width: '100%',
          maxWidth: '480px',
          padding: '32px 24px',
          background: 'var(--bg-surface)',
          border: '1px solid var(--border)',
          borderRadius: 'var(--radius-xl)',
          boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '24px',
        }}>
          {status === 'receiving' && transfer ? (
            <>
              {/* Spinner Icon wrapper */}
              <div style={{
                width: 72, height: 72,
                borderRadius: '50%',
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                position: 'relative'
              }}>
                <div style={{
                  position: 'absolute', top: 0, left: 0, right: 0, bottom: 0,
                  borderRadius: '50%',
                  border: '3px solid var(--border)',
                  borderTop: '3px solid var(--accent)',
                  animation: 'spin-anim 1.5s linear infinite'
                }} />
                <FileUp size={28} style={{ color: 'var(--text-secondary)' }} />
              </div>

              {/* Title & Filename */}
              <div style={{ textAlign: 'center' }}>
                <h3 style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>
                  {transfer.status === 'Decoding' ? 'Decoding File...' :
                   transfer.status === 'Verifying' ? 'Verifying Integrity...' :
                   transfer.status === 'Saving' ? 'Saving to Disk...' :
                   'Receiving File...'}
                </h3>
                <p style={{ fontSize: 14, color: 'var(--text-secondary)', wordBreak: 'break-all' }}>{transfer.filename}</p>
              </div>

              {/* Progress Bar Container */}
              <div style={{ width: '100%' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 8 }}>
                  <span>{percent}%</span>
                  <span>{formatBytes(transfer.bytes_received)} / {formatBytes(transfer.total_bytes)}</span>
                </div>
                <div style={{
                  width: '100%', height: 6,
                  background: 'var(--bg-elevated)',
                  borderRadius: 3,
                  overflow: 'hidden',
                  border: '1px solid var(--border)'
                }}>
                  <div style={{
                    width: `${percent}%`, height: '100%',
                    background: 'var(--accent)',
                    borderRadius: 3,
                    transition: 'width 0.1s ease-out'
                  }} />
                </div>
              </div>

              {/* Transfer Metrics Grid */}
              <div style={{
                display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16,
                width: '100%', padding: '16px',
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border)',
                borderRadius: 'var(--radius-lg)'
              }}>
                <div>
                  <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 2 }}>SPEED</div>
                  <div style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)' }}>
                    {isValidating ? '-' : formatSpeed(transfer.speed_bytes_per_sec)}
                  </div>
                </div>
                <div>
                  <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 2 }}>ESTIMATED TIME</div>
                  <div style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)' }}>
                    {isValidating
                      ? (transfer.status === 'Decoding' ? 'Decoding...' : transfer.status === 'Verifying' ? 'Verifying...' : 'Saving...')
                      : (eta !== null ? `${eta}s` : 'Calculating...')}
                  </div>
                </div>
              </div>
            </>
          ) : status === 'completed' ? (
            <>
              {/* Success Icon */}
              <div style={{
                width: 72, height: 72,
                borderRadius: '50%',
                background: 'rgba(52,211,153,0.1)',
                border: '1px solid var(--success)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
              }}>
                <CheckCircle size={32} style={{ color: 'var(--success)' }} />
              </div>

              <div style={{ textAlign: 'center' }}>
                <h3 style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>Transfer Completed</h3>
                <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>The file was saved to your Downloads folder.</p>
              </div>

              <button
                className="btn btn-primary"
                onClick={() => navigate({ to: '/' })}
                style={{ width: '100%', justifyContent: 'center', padding: '12px' }}
              >
                Back to Dashboard
              </button>
            </>
          ) : (
            <>
              {/* Idle State */}
              <div style={{
                width: 72, height: 72,
                borderRadius: '50%',
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
              }}>
                <WifiOff size={28} style={{ color: 'var(--text-tertiary)' }} />
              </div>

              <div style={{ textAlign: 'center' }}>
                <h3 style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>No Active Transfer</h3>
                <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>Ready to receive files from paired Android devices.</p>
              </div>

              <button
                className="btn btn-ghost"
                onClick={() => navigate({ to: '/' })}
                style={{ width: '100%', justifyContent: 'center', padding: '12px', background: 'rgba(255, 255, 255, 0.05)' }}
              >
                Back to Dashboard
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
