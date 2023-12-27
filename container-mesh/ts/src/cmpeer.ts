import { existsSync, mkdirSync } from 'node:fs'
import { Ditto, DocumentID, TransportConfig } from '@dittolive/ditto'
import { random_peer_id } from './util'
import { LatencyStats, PeerReport } from './types'


export enum CmeshEvent {
    BeginTest,
    EndTest,
    Exiting
};

export interface PeerArgs {
    // coordinator's ip address
    coord_addr: string
    coord_port: number | null
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

enum PeerState {
    Init, Ready, Running, Reporting, Shutdown
}

export type PeerId = string;

class PeerContext {
    public id: PeerId
    //ditto: Ditto | null,
    public coord_addr: string
    public coord_doc_id: DocumentID | null
    /*
    transport_config?: TransportConfig,
    hb_doc_id?: DocumentID,
    hb_ctx?: HeartbeatCtx,
    start_time_msec: number,
    local_ip: string,
    local_port: number,
    state: PeerState,
    peer_collection?: Collection,
    peer_consumer?: PeerConsumer,
    */

    constructor(id: PeerId, coord_addr: string) {
            this.id = id
            this.coord_addr = coord_addr
            this.coord_doc_id = null
    }
}

// TODO
type TestReport = string;

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

    async run_test(pctx: PeerContext) : Promise<PeerReport> {
        // TODO
        return new PeerReport()
    }

    // Start the peer and supply a callback for state transitions.
    public async start(cb: CMEventCallback) {

        // Check if output directory exists
        if (!existsSync(this.pargs.output_dir)) {
            console.log(`Creating output directory ${this.pargs.output_dir}`)
            mkdirSync(this.pargs.output_dir);
        }

        let pctx = new PeerContext(random_peer_id(this.pargs.peer_name), this.pargs.coord_addr)

        // bootstrap peer
        await this.bootstrap_peer(pctx)

        // TODO info
        console.log("--> Running test plan..");
        cb(CmeshEvent.BeginTest)
        let report = this.run_test(pctx)
        // XXX temporary for test: sleep for 1 second
        await new Promise(resolve => setTimeout(resolve, 100));
        console.log(report)
        await cb(CmeshEvent.EndTest)
        await new Promise(resolve => setTimeout(resolve, 100));
        await cb(CmeshEvent.Exiting)
    }

    async bootstrap_peer(pctx: PeerContext) {
        // TODO
        console.log("--> Bootstrap peer..")
        return Promise.resolve(null)
    }

}
