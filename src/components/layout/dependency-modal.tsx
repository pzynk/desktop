import { useState } from 'react'
import {
  Mic,
  Camera,
  Volume2,
  Copy,
  Check,
  X,
  Loader2,
  ShieldCheck,
  AlertCircle,
  ChevronDown,
  ChevronUp,
  Sparkles,
  Layers,
} from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'

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
  onSuccess?: () => void
  onStreamDirect?: () => void
}

export function DependencyModal({
  dependency,
  onClose,
  onSuccess,
  onStreamDirect,
}: DependencyModalProps) {
  const [copied, setCopied] = useState(false)
  const [showTerminal, setShowTerminal] = useState(false)
  const [installing, setInstalling] = useState(false)
  const [installError, setInstallError] = useState<string | null>(null)
  const [installSuccess, setInstallSuccess] = useState(false)

  const isMic = dependency.feature === 'mic'
  const isCamera = dependency.feature === 'camera'

  const featureLabel = isMic ? 'Microphone' : isCamera ? 'Virtual Camera' : 'Device'
  const setupButtonLabel = isMic ? 'Setup Microphone' : isCamera ? 'Setup Virtual Camera' : 'Setup Driver'

  const handleCopy = () => {
    navigator.clipboard.writeText(dependency.command)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleAutoInstall = async () => {
    setInstalling(true)
    setInstallError(null)
    try {
      await invoke('auto_install_system_dep', { feature: dependency.feature })
      setInstallSuccess(true)
      setTimeout(() => {
        onSuccess?.()
        onClose()
      }, 1000)
    } catch (e: any) {
      console.error('Auto install failed:', e)
      setInstallError(typeof e === 'string' ? e : e?.message || 'Setup encountered an issue. You can try the manual command below.')
    } finally {
      setInstalling(false)
    }
  }

  return (
    <div className="modal-backdrop" style={{ backdropFilter: 'blur(8px)', background: 'rgba(0, 0, 0, 0.65)' }}>
      <div
        className="modal-card"
        style={{
          width: '500px',
          maxWidth: '92vw',
          borderRadius: '18px',
          background: 'var(--bg-surface, #18181b)',
          border: '1px solid rgba(255, 255, 255, 0.1)',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.05)',
          padding: '24px',
          position: 'relative',
        }}
      >
        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: '18px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
            <div
              style={{
                width: 48,
                height: 48,
                borderRadius: '14px',
                background: isMic
                  ? 'linear-gradient(135deg, rgba(168, 85, 247, 0.2), rgba(99, 102, 241, 0.2))'
                  : isCamera
                  ? 'linear-gradient(135deg, rgba(59, 130, 246, 0.2), rgba(14, 165, 233, 0.2))'
                  : 'linear-gradient(135deg, rgba(16, 185, 129, 0.2), rgba(5, 150, 105, 0.2))',
                border: isMic
                  ? '1px solid rgba(168, 85, 247, 0.3)'
                  : isCamera
                  ? '1px solid rgba(59, 130, 246, 0.3)'
                  : '1px solid rgba(16, 185, 129, 0.3)',
                color: isMic ? '#c084fc' : isCamera ? '#60a5fa' : '#34d399',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              {isMic ? <Mic width={24} height={24} /> : isCamera ? <Camera width={24} height={24} /> : <Volume2 width={24} height={24} />}
            </div>
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <span
                  style={{
                    fontSize: '11px',
                    fontWeight: 600,
                    textTransform: 'uppercase',
                    letterSpacing: '0.05em',
                    padding: '2px 8px',
                    borderRadius: '6px',
                    background: 'rgba(255, 255, 255, 0.08)',
                    color: 'var(--text-secondary, #a1a1aa)',
                  }}
                >
                  {featureLabel} Setup
                </span>
              </div>
              <div className="modal-title" style={{ fontSize: '18px', fontWeight: 650, margin: '4px 0 0 0', color: 'var(--text-primary, #fafafa)' }}>
                {dependency.title}
              </div>
            </div>
          </div>

          <button
            onClick={onClose}
            disabled={installing}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-tertiary, #71717a)',
              cursor: 'pointer',
              padding: '6px',
              borderRadius: '8px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              transition: 'all 0.15s ease',
            }}
            title="Close"
          >
            <X size={18} />
          </button>
        </div>

        {/* Feature Explanation */}
        <div
          style={{
            fontSize: '13.5px',
            color: 'var(--text-secondary, #d4d4d8)',
            marginBottom: '16px',
            lineHeight: '1.55',
          }}
        >
          {dependency.description}
        </div>

        {/* Highlights / Features Banner */}
        <div
          style={{
            background: 'rgba(255, 255, 255, 0.03)',
            border: '1px solid rgba(255, 255, 255, 0.06)',
            borderRadius: '12px',
            padding: '12px 14px',
            marginBottom: '16px',
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12.5px', color: 'var(--text-secondary, #d4d4d8)' }}>
            <Sparkles size={15} color={isMic ? '#c084fc' : '#60a5fa'} style={{ flexShrink: 0 }} />
            <span>
              {isMic
                ? 'Creates a virtual input device for Google Meet, Zoom, Teams & Discord.'
                : 'Registers the virtual webcam driver for browsers and meeting apps.'}
            </span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12.5px', color: 'var(--text-tertiary, #a1a1aa)' }}>
            <Layers size={15} style={{ flexShrink: 0 }} />
            <span>Requires one-time driver setup with administrator permissions.</span>
          </div>
        </div>

        {/* Status Alerts */}
        {installError && (
          <div
            style={{
              background: 'rgba(239, 68, 68, 0.12)',
              border: '1px solid rgba(239, 68, 68, 0.3)',
              borderRadius: '10px',
              padding: '11px 14px',
              marginBottom: '16px',
              display: 'flex',
              alignItems: 'flex-start',
              gap: '10px',
              color: '#f87171',
              fontSize: '13px',
              lineHeight: '1.45',
            }}
          >
            <AlertCircle size={17} style={{ flexShrink: 0, marginTop: '2px' }} />
            <div>{installError}</div>
          </div>
        )}

        {installSuccess && (
          <div
            style={{
              background: 'rgba(16, 185, 129, 0.12)',
              border: '1px solid rgba(16, 185, 129, 0.3)',
              borderRadius: '10px',
              padding: '11px 14px',
              marginBottom: '16px',
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              color: '#34d399',
              fontSize: '13px',
            }}
          >
            <ShieldCheck size={18} />
            <div>Setup completed successfully! Starting {featureLabel.toLowerCase()} stream...</div>
          </div>
        )}

        {installing && (
          <div
            style={{
              background: 'rgba(59, 130, 246, 0.1)',
              border: '1px solid rgba(59, 130, 246, 0.25)',
              borderRadius: '10px',
              padding: '12px 14px',
              marginBottom: '16px',
              display: 'flex',
              alignItems: 'center',
              gap: '12px',
              fontSize: '13px',
              color: 'var(--text-primary, #fafafa)',
            }}
          >
            <Loader2 size={18} className="spin" color="#60a5fa" />
            <div>
              <div style={{ fontWeight: 600 }}>Setting up {featureLabel.toLowerCase()} driver...</div>
              <div style={{ fontSize: '12px', color: 'var(--text-tertiary, #a1a1aa)', marginTop: '2px' }}>
                Please approve the Windows administrator prompt if it pops up.
              </div>
            </div>
          </div>
        )}

        {/* Collapsible Manual Command */}
        <div style={{ marginBottom: '18px' }}>
          <button
            onClick={() => setShowTerminal(!showTerminal)}
            style={{
              background: 'transparent',
              border: 'none',
              padding: '4px 0',
              color: 'var(--text-tertiary, #a1a1aa)',
              fontSize: '12px',
              fontWeight: 500,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
            }}
          >
            <span>{showTerminal ? 'Hide manual command' : 'View manual command'}</span>
            {showTerminal ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>

          {showTerminal && (
            <div style={{ marginTop: '8px' }}>
              <div
                style={{
                  position: 'relative',
                  background: 'var(--bg-base, #09090b)',
                  border: '1px solid var(--border, rgba(255,255,255,0.08))',
                  borderRadius: '8px',
                  padding: '10px 38px 10px 12px',
                  fontFamily: 'monospace',
                  fontSize: '11.5px',
                  color: 'var(--accent, #a855f7)',
                  wordBreak: 'break-all',
                  lineHeight: '1.4',
                }}
              >
                {dependency.command}
                <button
                  onClick={handleCopy}
                  style={{
                    position: 'absolute',
                    top: '6px',
                    right: '6px',
                    background: 'var(--bg-elevated, #27272a)',
                    border: '1px solid rgba(255, 255, 255, 0.1)',
                    borderRadius: '4px',
                    padding: '4px 6px',
                    color: 'var(--text-secondary, #d4d4d8)',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                  title="Copy Command"
                >
                  {copied ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Action Buttons */}
        <div
          className="modal-actions"
          style={{
            marginTop: '0',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            flexWrap: 'wrap',
            gap: '10px',
          }}
        >
          <div>
            {isMic && onStreamDirect && (
              <button
                className="btn btn-ghost"
                style={{
                  fontSize: '12.5px',
                  padding: '8px 14px',
                  borderRadius: '10px',
                  border: '1px solid rgba(255, 255, 255, 0.1)',
                  color: 'var(--text-secondary, #d4d4d8)',
                }}
                onClick={() => {
                  onStreamDirect()
                  onClose()
                }}
              >
                <Volume2 size={15} /> Play to PC Speakers
              </button>
            )}
          </div>

          <div style={{ display: 'flex', gap: '8px' }}>
            <button
              className="btn btn-ghost"
              onClick={onClose}
              disabled={installing}
              style={{ padding: '8px 16px', borderRadius: '10px' }}
            >
              Cancel
            </button>

            <button
              className="btn btn-primary"
              style={{
                background: isMic
                  ? 'linear-gradient(135deg, #9333ea, #7c3aed)'
                  : 'linear-gradient(135deg, #2563eb, #1d4ed8)',
                color: '#fff',
                padding: '9px 18px',
                borderRadius: '10px',
                fontWeight: 600,
                fontSize: '13.5px',
                minWidth: '150px',
                boxShadow: isMic
                  ? '0 4px 14px rgba(147, 51, 234, 0.35)'
                  : '0 4px 14px rgba(37, 99, 235, 0.35)',
              }}
              onClick={handleAutoInstall}
              disabled={installing || installSuccess}
            >
              {installing ? (
                <>
                  <Loader2 size={16} className="spin" /> Setting up...
                </>
              ) : installSuccess ? (
                <>
                  <Check size={16} /> Configured
                </>
              ) : (
                <>
                  {isMic ? <Mic size={16} /> : isCamera ? <Camera size={16} /> : <Sparkles size={16} />}
                  {setupButtonLabel}
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
