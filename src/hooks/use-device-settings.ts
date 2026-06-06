import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { invoke } from '@tauri-apps/api/core'
import { TrustedPeer } from '../utils/device'

export function useDeviceSettings(id: string) {
  const navigate = useNavigate()
  const [peer, setPeer] = useState<TrustedPeer | null>(null)

  const refreshPeer = useCallback(() => {
    invoke<TrustedPeer[]>('list_trusted_peers')
      .then((peers) => {
        const found = peers.find((p) => p.device_id === id)
        if (found) {
          setPeer(found)
        } else {
          navigate({ to: '/' })
        }
      })
      .catch(() => navigate({ to: '/' }))
  }, [id, navigate])

  useEffect(() => {
    refreshPeer()
    const interval = setInterval(refreshPeer, 2000)
    return () => clearInterval(interval)
  }, [refreshPeer])

  const unpairPeer = useCallback(async () => {
    await invoke('unpair_peer', { deviceId: id }).catch(() => {})
    navigate({ to: '/' })
  }, [id, navigate])

  const toggleClipboardSync = useCallback(async () => {
    if (!peer) return
    await invoke('set_device_clipboard_sync', {
      deviceId: id,
      enabled: !peer.clipboard_sync_enabled,
    }).catch(() => {})
    refreshPeer()
  }, [id, peer, refreshPeer])

  const toggleMediaControls = useCallback(async () => {
    if (!peer) return
    await invoke('set_device_media_controls', {
      deviceId: id,
      enabled: !peer.media_controls_enabled,
    }).catch(() => {})
    refreshPeer()
  }, [id, peer, refreshPeer])

  const toggleVolumeSync = useCallback(async () => {
    if (!peer) return
    await invoke('set_device_volume_sync', {
      deviceId: id,
      enabled: !peer.volume_sync_enabled,
    }).catch(() => {})
    refreshPeer()
  }, [id, peer, refreshPeer])

  const toggleIncomingFiles = useCallback(async () => {
    if (!peer) return
    await invoke('set_device_incoming_files', {
      deviceId: id,
      enabled: !peer.incoming_files_enabled,
    }).catch(() => {})
    refreshPeer()
  }, [id, peer, refreshPeer])

  const toggleTerminalAccess = useCallback(async () => {
    if (!peer) return
    await invoke('set_device_terminal_access', {
      deviceId: id,
      enabled: !peer.terminal_access_enabled,
    }).catch(() => {})
    refreshPeer()
  }, [id, peer, refreshPeer])

  return {
    peer,
    unpairPeer,
    toggleClipboardSync,
    toggleMediaControls,
    toggleVolumeSync,
    toggleIncomingFiles,
    toggleTerminalAccess,
  }
}
