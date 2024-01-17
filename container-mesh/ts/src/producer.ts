import { Collection, MutableDocument } from '@dittolive/ditto'
import { PeerContext } from './context'
import { ExecutionPlan, PeerId, PeerRecord } from './types'
import { PEER_LOG_SIZE } from './default'

export class Producer {
    peerId: PeerId
    collection: Collection
    plan: ExecutionPlan
    msgIndex: number
    msgCount: number
    finished: boolean
    timer: NodeJS.Timeout | null

    constructor(pctx: PeerContext, peerColl: Collection) {
        this.peerId = pctx.id
        this.plan = pctx.coord_info!.execution_plan!
        this.collection = peerColl
        this.msgIndex = 0
        this.msgCount = 0
        this.finished = true
        this.timer = null
    }

    randDelay(): number {
        const range = this.plan.max_msg_delay_msec - this.plan.min_msg_delay_msec
        return Math.floor(this.plan.min_msg_delay_msec + Math.random() * range)
    }

    getNextIdx(): number {
        this.msgIndex += 1
        if (this.msgIndex >= PEER_LOG_SIZE) {
            this.msgIndex = 0
        }
        return this.msgIndex
    }

    async setProduceTimer(): Promise<void> {
        const nextDelay = this.randDelay()
        console.debug(`--> setting produce timer: ${nextDelay} msec`)
        this.timer = setTimeout(() => { this.produce() }, nextDelay)
    }

    async produce(): Promise<void> {
        if (this.finished) {
            return
        }
        console.debug(`--> producing message ${this.msgCount}`)
        const next_index = this.getNextIdx()
        const rec = new PeerRecord()
        const idOp = this.collection.findByID(this.plan.peer_doc_id)
        const recPath = `logs['${this.peerId}']['log']['${next_index}']`
        idOp.update((mutDoc: MutableDocument) => {
            const dp = mutDoc.at(recPath)
            dp.set(rec)
        })
        return this.setProduceTimer()
    }

    async start(): Promise<void> {
        console.info("--> producer start")
        this.finished = false
        return this.setProduceTimer()
    }

    // Stop producer and return number of records produced
    async stop(): Promise<number> {
        console.info("--> producer stop")
        this.finished = true
        return this.msgCount
    }
}
