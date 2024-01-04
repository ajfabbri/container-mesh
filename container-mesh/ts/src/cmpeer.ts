import { existsSync, mkdirSync } from 'node:fs'
import { Collection, DocumentID, PendingCursorOperation, Subscription, TransportConfig } from '@dittolive/ditto'
import { make_ditto, random_peer_id } from './util'
import { Heartbeat, PeerReport, PeerState } from './types'
import { REPORT_PROPAGATION_SEC } from './default'
import { PeerContext } from './context'


export enum CmeshEvent {
    BeginTest,
    EndTest,
    Exiting
}

export interface PeerArgs {
    // coordinator's ip address
    coord_addr: string
    coord_port: number
    peer_name: string
    bind_addr: string
    bind_port: number
    output_dir: string
}

export const defaultPeerArgs: PeerArgs = {
    coord_addr: "127.0.0.1",
    coord_port: 4000,
    peer_name: "ts-peer",
    bind_addr: "0.0.0.0",
    bind_port: 4010,
    output_dir: "output"
}


type CMEventCallback = (event: CmeshEvent) => Promise<void>;
export class CmeshPeer {
    pargs: PeerArgs;

    //ditto: Ditto;
    hello(who: string): void {
        console.log(`Hello ${who}!`)
    }
    constructor(args: PeerArgs) {
        this.pargs = args;
    }

    async runTest(pctx: PeerContext): Promise<PeerReport> {

        console.log("TODO connect mesh")

        // wait for start time
        const start_time = pctx.coord_info!.executionPlan!.start_time
        const delay = start_time - Date.now()
        if (delay > 0) {
            console.log(`--> Waiting ${delay} msec for start time`)
            await new Promise(resolve => setTimeout(resolve, delay))
        }

        pctx.stateTransition(PeerState.Ready, PeerState.Running)

        // TODO create consumer
        console.log("TODO create consumer")
        // TODO create producer
        console.log("TODO create producer")
        // TODO wait for test duration
        const dur = pctx.coord_info!.executionPlan!.test_duration_sec
        console.log(`--> Waiting for test duration (${dur} sec)`)
        await new Promise(resolve => setTimeout(resolve, dur))

        pctx.stateTransition(PeerState.Running, PeerState.Reporting)
        // TODO stop producer
        // TODO grab test results
        return new PeerReport()
    }

    // Start the peer and supply a callback for state transitions.
    public async start(cb: CMEventCallback) {

        // Check if output directory exists
        if (!existsSync(this.pargs.output_dir)) {
            console.log(`Creating output directory ${this.pargs.output_dir}`)
            mkdirSync(this.pargs.output_dir);
        }
        const ditto = make_ditto()
        const pctx = new PeerContext(random_peer_id(this.pargs.peer_name), ditto, this.pargs.coord_addr,
                                   this.pargs.coord_port, this.pargs.bind_addr, this.pargs.bind_port)

        // bootstrap peer
        await this.bootstrapPeer(pctx)

        // TODO info
        console.log("--> Running test plan..")
        await cb(CmeshEvent.BeginTest)
        const report = this.runTest(pctx)
        await cb(CmeshEvent.EndTest)
        await new Promise(resolve => setTimeout(resolve, REPORT_PROPAGATION_SEC))
        console.log(report)
        pctx.stateTransition(PeerState.Reporting, PeerState.Shutdown)
        await cb(CmeshEvent.Exiting)
    }

    async initTransport(pctx: PeerContext) {
        // Default config has all transports disabled
        const config = new TransportConfig()
        config.peerToPeer.lan.isEnabled = true
        // TODO resolve / validate hostname
        config.connect.tcpServers = [`${pctx.coord_addr}:${pctx.coord_port}`]
        config.connect.websocketURLs = []
        config.listen.tcp.isEnabled = true
        config.listen.tcp.port = pctx.coord_port
        config.listen.tcp.isEnabled = true
        config.listen.tcp.interfaceIP = pctx.local_addr
        config.listen.tcp.port = pctx.local_port
        console.log(`--> set transport config listen ${config.listen.tcp.interfaceIP}:${config.listen.tcp.port}`)
        pctx.ditto!.setTransportConfig(config)
    }

    async initLicense(pctx: PeerContext) {
        const lkey = process.env.DITTO_LICENSE
        if (!lkey) {
            throw new Error("DITTO_LICENSE environment variable not set")
        }
        pctx.ditto?.setOfflineOnlyLicenseToken(lkey)
    }

    // Resolves when coord info has been fetched and set in pctx
    async getCoordInfo(pctx: PeerContext, coll: Collection, needPlan: boolean, needStart: boolean) {
        // Return a promise that resolves once we receive a non-empty coord info doc
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        return new Promise<void>((resolve, _reject) => {
            coll.findAll().observeLocal((info, event) => {
                if (event.isInitial) {
                    return
                }
                if (info.length > 0) {
                    // XXX debug log
                    const cinfo = info[0]
                    pctx.coord_info = {
                        heartbeatCollectionName: cinfo.at('heartbeat_collection_name').value(),
                        heartbeatIntervalSec: cinfo.at('heartbeat_interval_sec').value(),
                        executionPlan: cinfo.at('execution_plan').value(),
                    }
                    if ((needPlan || needStart) && !pctx.coord_info.executionPlan) {
                        return
                    }
                    if (needStart && !pctx.coord_info.executionPlan?.start_time) {
                        return
                    }

                    console.log(`--> coord info: ${pctx.coord_info}`)
                    resolve()
                }
            })
        })
    }

    async getInitialCoordInfo(pctx: PeerContext, coll: Collection) {
        return this.getCoordInfo(pctx, coll, false, false)
    }

    async getExecutionPlan(pctx: PeerContext, coll: Collection, needStartTime: boolean) {
        return this.getCoordInfo(pctx, coll, true, needStartTime)
    }

    async startHeartbeat(pctx: PeerContext): Promise<NodeJS.Timeout> {
        const hbc = pctx.ditto!.store.collection(pctx.coord_info!.heartbeatCollectionName)
        const hb_query: PendingCursorOperation = hbc.findAll()
        const hb_sub: Subscription = hb_query.subscribe()
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const initial_doc = new Promise<DocumentID>((resolve, _reject) => {
            // eslint-disable-next-line @typescript-eslint/no-unused-vars
            hb_query.observeLocal((docs, _event) => {
                if (docs.length > 0) {
                    resolve(docs[0].id)
                    hb_sub.cancel()
                }
            })
        })
        const doc_id = await initial_doc
        // set self-refreshing heartbeat send timer
        const hb_func = async () => {
            // heartbeats are used only for bootstrapping, not during actual test run
            if (pctx.state != PeerState.Init && pctx.state != PeerState.Ready) {
                return
            }
            const hb: Heartbeat = { sender: pctx.toPeer(),
                sent_at_usec: Date.now() * 1000 }
            hbc.findByID(doc_id).update( (mutDoc) => {
                mutDoc.at('beats').set(hb)
            })
        }
        return setInterval(hb_func, pctx.coord_info!.heartbeatIntervalSec * 1000)
    }

    async bootstrapPeer(pctx: PeerContext) {
        this.initTransport(pctx)
        this.initLicense(pctx)
        const store = pctx.ditto!.store
        const coord_coll = store.collection(pctx.coord_collection)
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const _coord_sub = coord_coll.findAll().subscribe()

        await this.getInitialCoordInfo(pctx, coord_coll)
        const hb_timer = await this.startHeartbeat(pctx)

        await this.getExecutionPlan(pctx, coord_coll, false)
        // signal to coord that we are ready to execute
        pctx.stateTransition(PeerState.Init, PeerState.Ready)
        await this.getExecutionPlan(pctx, coord_coll, true)
        clearInterval(hb_timer)
    }

}
