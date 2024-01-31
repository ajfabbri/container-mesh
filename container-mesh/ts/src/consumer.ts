import { Collection, DocumentPath, LiveQuery, Subscription, DocumentID } from '@dittolive/ditto'
import { PeerContext } from './context.js'
import { LatencyStats, PeerId, PeerDoc, PeerLogs, PeerLog, PeerRecord } from './types.js'
import { system_time_msec } from './util.js'
import { PEER_LOG_SIZE } from './default.js'

interface TimestampIndex {
    ts: number
    i: number
}

export class Consumer {
    pctx: PeerContext
    running: boolean
    lastTsIdxByPeer: Map<PeerId, TimestampIndex>
    msgLatency: LatencyStats
    msgLatencyTotal: number
    collection: Collection
    liveQuery: LiveQuery | null
    timeout: NodeJS.Timeout | null
    sub: Subscription | null


    constructor(pctx: PeerContext, peerColl: Collection) {
        this.pctx = pctx
        this.running = false
        this.lastTsIdxByPeer = new Map<PeerId, TimestampIndex>()
        this.msgLatency = new LatencyStats()
        this.msgLatencyTotal = 0
        this.collection = peerColl
        this.liveQuery = null
        this.timeout = null
        this.sub = null
    }

    async createPeerCollection(): Promise<Collection> {
        const store = this.pctx.ditto!.store
        const plan = this.pctx.coord_info!.execution_plan!
        const pc = store.collection(plan.peer_collection_name)
        const log : {[key: string]: PeerRecord} = {}
        const logs : {[key: string]: PeerLog} = {}
        logs[this.pctx.id] = { log: log }
        const pdoc: PeerDoc = { _id: plan.peer_doc_id, logs: logs }
        await pc.upsert(pdoc)
        return pc
    }

    async processPeer(pid: PeerId, plog: PeerLog): Promise<void> {
        const now = system_time_msec()
        let {ts, i} = this.getTSIdx(pid)
        // eslint-disable-next-line no-constant-condition
        while (true) {
            const rec = plog.log[i.toString()]
            if (!rec || rec.timestamp < ts) {
                // no record, or we wrapped around to an old one
                break;
            }
            const latency = now - rec.timestamp
            this.msgLatencyTotal += latency
            this.msgLatency.num_events += 1
            this.msgLatency.min_msec = Math.min(this.msgLatency.min_msec, latency)
            this.msgLatency.max_msec = Math.max(this.msgLatency.max_msec, latency)
            this.msgLatency.avg_msec = this.msgLatencyTotal / this.msgLatency.num_events
            console.debug('--> got peer record ', rec, ` w/ latency ${latency}`)
            i = incrWrap(i, PEER_LOG_SIZE-1)
            ts = rec.timestamp
        }
        this.setConsumedTSIdx(pid, ts, i, PEER_LOG_SIZE-1)
    }

    // get timestamp of last record consumed, and expected next index
    getTSIdx(pid: PeerId): TimestampIndex {
        let r = this.lastTsIdxByPeer.get(pid)
        if (!r) {
            r = { ts: 0, i: 0 }
        }
        return r
    }

    // set last timestamp and log index we consumed for this peer
    setConsumedTSIdx(pid: PeerId, ts: number, idx: number, maxIdx: number) {
        const i = incrWrap(idx, maxIdx)
        this.lastTsIdxByPeer.set(pid, { ts: ts, i: i })
    }

    // Start consumer and return the peer collection
    async start(): Promise<Collection> {
        const plan = this.pctx.coord_info!.execution_plan!
        const docId: DocumentID = plan.peer_doc_id
        const docIdStr = docId.toQueryCompatibleString
        console.info(`--> consumer start for coll ${plan.peer_collection_name}` +
                     `w/ doc id ${docIdStr} logged: `, docId)
        this.running = true
        const pc = await this.createPeerCollection()
        const query = pc.findByID(plan.peer_doc_id)
        this.sub = query.subscribe()
        /* XXX TODO use DQL
        this.timeout = setInterval(async () => {
            const result = await this.pctx.ditto!.store.execute(
                `SELECT * FROM COLLECTION "${plan.peer_collection_name}" (logs MAP) WHERE _id = "${plan.peer_doc_id}"`)
            for (const item of result.items) {
                console.debug(`--> query result ${item.jsonString()}`)
                console.debug(`--> query result ${stringify(item)}`)
            }
        }, 1000)
        */

        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        this.liveQuery = query.observeLocal(async (doc, _event) => {
            if (!this.running) {
                return
            }
            if (doc) {
                const logsPath: DocumentPath = doc.at('logs')
                const logs: PeerLogs = logsPath.value
                if (!logs) {
                    return;
                }
                for (const peerId of Object.keys(logs)) {
                    if (peerId == this.pctx.id) {
                        continue
                    }
                    this.processPeer(peerId, logs[peerId])
                }
            }
        })
        return pc
    }

    async stop(): Promise<LatencyStats> {
        console.info("--> consumer stop")
        this.running = false
        if (this.liveQuery) {
            this.liveQuery!.stop()
        }
        if (this.timeout) {
            clearInterval(this.timeout)
        }
        this.msgLatency.distinct_peers = this.lastTsIdxByPeer.size
        return this.msgLatency
    }

}

function incrWrap(i: number, max: number): number {
    let r = i + 1
    if (r > max) {
        r = 0
    }
    return r
}
