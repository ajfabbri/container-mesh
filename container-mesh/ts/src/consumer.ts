import { LiveQuery, Subscription } from '@dittolive/ditto'
import { PeerContext } from './context'
import { LatencyStats, PeerId } from './types'

interface TimestampIndex {
    timestamp: number
    index: number
}

export class Consumer {
    pctx: PeerContext
    running: boolean
    lastTsIdxByPeer: Map<PeerId, TimestampIndex>
    msgLatency: LatencyStats
    msgLatencyTotal: number
    subscription: Subscription
    liveQuery: LiveQuery | null



    constructor(pctx: PeerContext, peerSub: Subscription) {
        this.pctx = pctx
        this.running = false
        this.lastTsIdxByPeer = new Map<PeerId, TimestampIndex>()
        this.msgLatency = new LatencyStats()
        this.msgLatencyTotal = 0
        this.subscription = peerSub
        this.liveQuery = null
    }

    async start(): Promise<LatencyStats> {
        console.info("--> consumer start")
        // TODO
        return this.msgLatency
    }

    async stop() {
        console.info("--> consumer stop")
        this.running = false
    }

}
