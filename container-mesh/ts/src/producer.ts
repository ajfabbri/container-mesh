import { PeerContext } from './context'

export class Producer {
    pctx: PeerContext
    msgIndex: number
    msgCount: number
    running: boolean
    timer: NodeJS.Timeout | null

    constructor(pctx: PeerContext) {
        this.pctx = pctx
        this.msgIndex = 0
        this.msgCount = 0
        this.running = false
        this.timer = null
    }

    async start(): Promise<void> {
        console.info("--> producer start")
        this.running = true
        /*
        const range = plan.max_msg_delay_msec - plan.min_msg_delay_msec
        const nextDelay = plan.min_msg_delay_msec + Math.random() * range
        setTimeout(() => {
        */
    }

    // Stop producer and return number of records produced
    async stop(): Promise<number> {
        console.info("--> producer stop")
        this.running = false
        return this.msgCount
    }
}
