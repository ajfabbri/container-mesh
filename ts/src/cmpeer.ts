import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { Collection, DocumentID, PendingCursorOperation, Subscription, TransportConfig } from '@dittolive/ditto'
import { make_ditto, random_peer_id, system_time_msec} from './util.js'
import { CoordinatorInfo, Heartbeat, Peer, PeerReport, PeerState } from './types.js'
import { QUERY_POLL_SEC, REPORT_PROPAGATION_SEC } from './default.js'
import { PeerContext } from './context.js'
import { Consumer } from './consumer.js'
import { Producer } from './producer.js'
import { PeerArgs } from './peerargs.js'

// State transitions exposed to the app using this library
export enum CmeshEvent {
    BeginTest,  // Test execution is about to begin
    EndTest,    // Test execution has finished
    Exiting     // Last chance to use CmeshPeer before it cleans up
}

type CMEventCallback = (event: CmeshEvent) => Promise<void>;
export class CmeshPeer {
    pargs: PeerArgs
    report: PeerReport | null = null

    constructor(args: PeerArgs) {
        this.pargs = args;
    }

    async connectMesh(pctx: PeerContext): Promise<void> {
        const plan = pctx.coord_info!.execution_plan!
        const peersToConnect = plan.connections[pctx.id]
        console.debug('--> initiating connections to', peersToConnect)
        for (const pid of peersToConnect) {
            const peerObj: Peer | undefined = plan.peers.find((p: Peer) => p.peer_id == pid)
            if (peerObj === undefined) {
                console.error(`--> Peer ${pid} not found in execution plan's peer list`)
                continue
            }
            const peer = `${peerObj!.peer_ip_addr}:${peerObj!.peer_port}`
            pctx.transport_config!.connect.tcpServers.push(peer)
        }
        pctx.ditto!.setTransportConfig(pctx.transport_config!)
    }

    async runTest(pctx: PeerContext): Promise<PeerReport> {

        const plan = pctx.coord_info!.execution_plan!
        this.connectMesh(pctx)

        // wait for start time
        const start_time = pctx.coord_info!.execution_plan!.start_time
        const delay = start_time - Date.now()
        if (delay > 0) {
            console.log(`--> Waiting ${delay} msec for start time`)
            await new Promise(resolve => setTimeout(resolve, delay))
        } else {
            console.debug(`--> Start time ${start_time} already passed (delta ${delay})`)
        }

        pctx.stateTransition(PeerState.Ready, PeerState.Running)

        const peerColl = pctx.ditto!.store.collection(plan.peer_collection_name)
        const peerSub: Subscription = peerColl.findAll().subscribe()

        const consumer = new Consumer(pctx, peerColl)
        await consumer.start()

        const producer = new Producer(pctx, peerColl)
        await producer.start()

        console.log(`--> Waiting for test duration (${plan.test_duration_sec} sec)`)
        await new Promise(resolve => setTimeout(resolve, plan.test_duration_sec * 1000))

        const recordsProduced = await producer.stop()
        const latency = await consumer.stop()

        pctx.stateTransition(PeerState.Running, PeerState.Reporting)
        peerSub.cancel()
        return new PeerReport(latency, recordsProduced)
    }

    public printReport() {
        if (this.report == null) {
            console.error("No report available yet!")
            return
        }
        const output: string = JSON.stringify(this.report!)
        output.replace(/\n/g, " ")
        // write to file in pargs.output_dir
        const fname = `${this.pargs.output_dir}/${this.pargs.device_name}-report.json`
        console.log(`--> Writing report to ${fname}`)
        // write to the file
        return writeFileSync(fname, output)
    }

    // Start the peer and supply a callback for state transitions.
    public async start(cb: CMEventCallback) {

        // Check if output directory exists
        if (!existsSync(this.pargs.output_dir)) {
            console.log(`Creating output directory ${this.pargs.output_dir}`)
            mkdirSync(this.pargs.output_dir);
        }
        const ditto = make_ditto(this.pargs.ditto_app_id)
        const pctx = new PeerContext(random_peer_id(this.pargs.device_name), ditto, this.pargs.coord_addr,
                                   this.pargs.coord_port, this.pargs.bind_addr, this.pargs.bind_port)

        // bootstrap peer
        await this.bootstrapPeer(pctx)

        // TODO info
        console.log("--> Running test plan..")
        await cb(CmeshEvent.BeginTest)
        this.report = await this.runTest(pctx)
        await cb(CmeshEvent.EndTest)
        pctx.stateTransition(PeerState.Reporting, PeerState.Shutdown)
        const update_wait = new Promise(resolve => setTimeout(resolve, REPORT_PROPAGATION_SEC * 1000))
        await cb(CmeshEvent.Exiting)
        await update_wait
        console.debug("--> Stopping heartbeat timer..")
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
        console.log(`--> set transport config listen ${config.listen.tcp.interfaceIP}:`
                   + `${config.listen.tcp.port}, coord ${coord_tcp}`)
        pctx.ditto!.setTransportConfig(config)
        pctx.transport_config = config
    }

    async initLicense(pctx: PeerContext) {
        let lkey
        if (this.pargs.ditto_license) {
            lkey = this.pargs.ditto_license
        } else {
            const lkey = process.env.DITTO_LICENSE
            if (!lkey) {
                throw new Error("Must set ditto_license in config, or "
                                + "DITTO_LICENSE in env.")
            }
        }
        pctx.ditto?.setOfflineOnlyLicenseToken(lkey!)
    }

    // Resolves when coord info has been fetched and set in pctx
    async getCoordInfo(pctx: PeerContext, coll: Collection, needPlan: boolean, needStart: boolean) {
        // Return a promise that resolves once we receive a non-empty coord info doc
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        return new Promise<void>((resolve, _reject) => {
            coll.findAll().observeLocal((docs) => {
                if (docs.length > 0) {
                    const cinfo = docs[0]
                    if (!cinfo.value) {
                        return
                    }
                    pctx.coord_info = cinfo.value as CoordinatorInfo
                    if (pctx.coord_info.heartbeat_collection_name == null) {
                        // Saw this once. :shrug:
                        console.error("--> coord_info has null hb collection name!")
                    }
                    if ((needPlan || needStart) && !pctx.coord_info.execution_plan) {
                        return
                    }
                    if (needStart && !pctx.coord_info.execution_plan?.start_time) {
                        return
                    }

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
         await this.getCoordInfo(pctx, coll, true, needStartTime)
         const plan = pctx.coord_info!.execution_plan!
         console.debug("--> getExecutionPlan:", plan)
         // XXX force deserialization
         plan.peer_doc_id = new DocumentID(plan.peer_doc_id)
         console.debug("--> getExecutionPlan after fixup:", plan)
    }

    async getHeartbeatDocId(pctx: PeerContext, hbc: Collection): Promise<DocumentID> {
        const hb_query: PendingCursorOperation = hbc.findAll()
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const _hb_sub: Subscription = hb_query.subscribe()
        console.debug(`Subscribed to ${pctx.coord_info!.heartbeat_collection_name}`)

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
        const hbc = pctx.ditto!.store.collection(pctx.coord_info!.heartbeat_collection_name)
        const doc_id = await this.getHeartbeatDocId(pctx, hbc)

        // set self-refreshing heartbeat send timer
        const hb_func = () => {
            // heartbeats are used only for bootstrapping, not during actual test run
            console.debug(`Heartbeat timer fired w/ state ${PeerState[pctx.state]}`)
            const hb: Heartbeat = { sender: pctx.toSerializedPeer(),
                sent_at_msec: system_time_msec() }
            const id_op = hbc.findByID(doc_id)
            id_op.update( (mutDoc) => {
                const beats = mutDoc.at(`beats`)
                console.debug(`beats: ${JSON.stringify(beats.value)}`)
                beats.at(pctx.id).set(hb)
            })
        }
        return setInterval(hb_func, pctx.coord_info!.heartbeat_interval_sec * 1000)
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
