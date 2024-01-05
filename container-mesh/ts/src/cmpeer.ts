import { existsSync, mkdirSync } from 'node:fs'
import { Collection, DocumentID, PendingCursorOperation, Subscription, TransportConfig } from '@dittolive/ditto'
import { make_ditto, random_peer_id} from './util'
import { Heartbeat, PeerReport, PeerState } from './types'
import { QUERY_POLL_SEC, REPORT_PROPAGATION_SEC } from './default'
import { PeerContext } from './context'
import { Consumer } from './consumer'
import { Producer } from './producer'


// State transitions exposed to the app using this library
export enum CmeshEvent {
    BeginTest,  // Test execution is about to begin
    EndTest,    // Test execution has finished
    Exiting     // Last chance to use CmeshPeer before it cleans up
}

export interface PeerArgs {
    // coordinator's ip address
    coord_addr: string
    coord_port: number
    device_name: string
    bind_addr: string
    bind_port: number
    output_dir: string
}

export const defaultPeerArgs: PeerArgs = {
    coord_addr: "127.0.0.1",
    coord_port: 4001,
    device_name: "tspeer",
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

        const plan = pctx.coord_info!.executionPlan!
        console.log("TODO connect mesh")

        // wait for start time
        const start_time = pctx.coord_info!.executionPlan!.start_time
        const delay = start_time - Date.now()
        if (delay > 0) {
            console.log(`--> Waiting ${delay} msec for start time`)
            await new Promise(resolve => setTimeout(resolve, delay))
        }

        pctx.stateTransition(PeerState.Ready, PeerState.Running)

        const peerColl = pctx.ditto!.store.collection(plan.peer_collection_name)
        const peerSub: Subscription = peerColl.findAll().subscribe()

        const consumer = new Consumer(pctx, peerColl)
        await consumer.start()

        const producer = new Producer(pctx)
        await producer.start()

        console.log(`--> Waiting for test duration (${plan.test_duration_sec} sec)`)
        await new Promise(resolve => setTimeout(resolve, plan.test_duration_sec * 1000))

        const recordsProduced = await producer.stop()
        const latency = await consumer.stop()

        pctx.stateTransition(PeerState.Running, PeerState.Reporting)
        peerSub.cancel()
        return new PeerReport(latency, recordsProduced)
    }

    // Start the peer and supply a callback for state transitions.
    public async start(cb: CMEventCallback) {

        // Check if output directory exists
        if (!existsSync(this.pargs.output_dir)) {
            console.log(`Creating output directory ${this.pargs.output_dir}`)
            mkdirSync(this.pargs.output_dir);
        }
        const ditto = make_ditto()
        const pctx = new PeerContext(random_peer_id(this.pargs.device_name), ditto, this.pargs.coord_addr,
                                   this.pargs.coord_port, this.pargs.bind_addr, this.pargs.bind_port)

        // bootstrap peer
        await this.bootstrapPeer(pctx)

        // TODO info
        console.log("--> Running test plan..")
        await cb(CmeshEvent.BeginTest)
        const report = await this.runTest(pctx)
        await cb(CmeshEvent.EndTest)
        console.log(report)
        pctx.stateTransition(PeerState.Reporting, PeerState.Shutdown)
        const update_wait = new Promise(resolve => setTimeout(resolve, REPORT_PROPAGATION_SEC * 1000))
        await cb(CmeshEvent.Exiting)
        await update_wait
        clearInterval(pctx.hb_timer!)   // stop reporting our state to coordinator
        pctx.ditto!.stopSync()
    }

    async initTransport(pctx: PeerContext) {
        const coord_tcp = `${pctx.coord_addr}:${pctx.coord_port}`
        // Default config has all transports disabled
        const config = new TransportConfig()
        config.peerToPeer.lan.isEnabled = true
        // TODO resolve / validate hostname
        config.connect.tcpServers = [coord_tcp]
        config.connect.websocketURLs = []
        config.listen.tcp.isEnabled = true
        config.listen.tcp.port = pctx.coord_port
        config.listen.tcp.isEnabled = true
        config.listen.tcp.interfaceIP = pctx.local_addr
        config.listen.tcp.port = pctx.local_port
        console.log(`--> set transport config listen ${config.listen.tcp.interfaceIP}:${config.listen.tcp.port}, coord ${coord_tcp}`)
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
            coll.findAll().observeLocal((docs) => {
                if (docs.length > 0) {
                    const cinfo = docs[0]
                    pctx.coord_info = {
                        heartbeatCollectionName: cinfo.at('heartbeat_collection_name').value,
                        heartbeatIntervalSec: cinfo.at('heartbeat_interval_sec').value,
                        executionPlan: cinfo.at('execution_plan').value,
                    }
                    if ((needPlan || needStart) && !pctx.coord_info.executionPlan) {
                        return
                    }
                    if (needStart && !pctx.coord_info.executionPlan?.start_time) {
                        return
                    }

                    console.log(`--> coord info: ${JSON.stringify(pctx.coord_info)}`)
                    resolve()
                }
            })
        })
    }

    async getInitialCoordInfo(pctx: PeerContext, coll: Collection) {
        console.debug("Getting intial coord info")
        return this.getCoordInfo(pctx, coll, false, false)
    }

    async getExecutionPlan(pctx: PeerContext, coll: Collection, needStartTime: boolean) {
        return this.getCoordInfo(pctx, coll, true, needStartTime)
    }

    async getHeartbeatDocId(pctx: PeerContext, hbc: Collection): Promise<DocumentID> {
        const hb_query: PendingCursorOperation = hbc.findAll()
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const _hb_sub: Subscription = hb_query.subscribe()
        console.debug(`Subscribed to ${pctx.coord_info!.heartbeatCollectionName}`)

        let doc_id: DocumentID | null = null
        while (doc_id == null) {
            const docs = await hb_query.exec()
            if (docs.length > 0) {
                doc_id = docs[0].id
            } else {
                // wait for 2 seconds
                await new Promise(resolve => setTimeout(resolve, QUERY_POLL_SEC))
            }
        }
        return doc_id
    }

    async startHeartbeat(pctx: PeerContext): Promise<NodeJS.Timeout> {
        const hbc = pctx.ditto!.store.collection(pctx.coord_info!.heartbeatCollectionName)
        const doc_id = await this.getHeartbeatDocId(pctx, hbc)

        // set self-refreshing heartbeat send timer
        const hb_func = () => {
            // heartbeats are used only for bootstrapping, not during actual test run
            console.debug(`Heartbeat timer fired w/ state ${PeerState[pctx.state]}`)
            const hb: Heartbeat = { sender: pctx.toSerializedPeer(),
                sent_at_usec: Date.now() * 1000 }
            const id_op = hbc.findByID(doc_id)
            id_op.update( (mutDoc) => {
                const beats = mutDoc.at(`beats`)
                console.debug(`beats: ${JSON.stringify(beats.value)}`)
                beats.at(pctx.id).set(hb)
            })
        }
        return setInterval(hb_func, pctx.coord_info!.heartbeatIntervalSec * 1000)
    }

    async bootstrapPeer(pctx: PeerContext) {
        this.initTransport(pctx)
        this.initLicense(pctx)
        pctx.ditto!.startSync()
        const coord_coll = pctx.ditto!.store.collection(pctx.coord_collection)
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const _coord_sub = coord_coll.findAll().subscribe()
        console.debug(`Subscribed to ${pctx.coord_collection}`)

        await this.getInitialCoordInfo(pctx, coord_coll)
        pctx.hb_timer = await this.startHeartbeat(pctx)

        await this.getExecutionPlan(pctx, coord_coll, false)
        // signal to coord that we are ready to execute
        pctx.stateTransition(PeerState.Init, PeerState.Ready)
        await this.getExecutionPlan(pctx, coord_coll, true)
    }

}
