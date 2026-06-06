import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { TrustedPeer, PairRequest } from '../utils/device'

export function usePeers() {
  const [peers, setPeers] = useState<TrustedPeer[]>([])
  const [pending, setPending] = useState<PairRequest[]>([])
  const [broadcasting, setBroadcasting] = useState(true)

  const refreshPeers = useCallback(() => {
    invoke<TrustedPeer[]>('list_trusted_peers')
      .then(setPeers)
      .catch(() => {})
  }, [])

  const resolvePairRequest = useCallback(async (deviceId: string, accepted: boolean) => {
    await invoke(accepted ? 'accept_pair_request' : 'reject_pair_request', { deviceId })
    setPending((prev) => prev.filter((r) => r.deviceId !== deviceId))
    if (accepted) {
      refreshPeers()
    }
  }, [refreshPeers])

  const toggleBroadcasting = useCallback(async () => {
    const next = !broadcasting
    setBroadcasting(next)
    await invoke('set_broadcasting', { enabled: next }).catch(() => {})
  }, [broadcasting])

  useEffect(() => {
    // Initial fetches
    invoke<PairRequest[]>('list_pending_pair_requests')
      .then(setPending)
      .catch(() => {})
    refreshPeers()
    invoke<boolean>('get_broadcasting')
      .then(setBroadcasting)
      .catch(() => {})

    // Event listeners
    const unlistenPromise = listen<PairRequest>('pair-request', (event) => {
      setPending((prev) => {
        const without = prev.filter((r) => r.deviceId !== event.payload.deviceId)
        return [...without, event.payload]
      })
    })

    const unlistenActivePromise = listen('active-connections-changed', () => {
      refreshPeers()
    })

    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {})
      unlistenActivePromise.then((fn) => fn()).catch(() => {})
    }
  }, [refreshPeers])

  return {
    peers,
    pending,
    broadcasting,
    refreshPeers,
    resolvePairRequest,
    toggleBroadcasting,
  }
}
