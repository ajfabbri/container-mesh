import { Ditto, DocumentID, TransportConfig } from "@dittolive/ditto"
import { CoordinatorInfo, Peer, PeerId } from "./types"
import { COORD_COLLECTION_NAME } from "./default"

export enum PeerState {
    Init, Ready, Running, Reporting, Shutdown
}

export class PeerContext {
    id: PeerId
    ditto: Ditto | null
    coord_addr: string
    coord_port: number
    coord_doc_id: DocumentID | null
    coord_collection: string
    coord_info: CoordinatorInfo | null
    transport_config: TransportConfig | null
    hb_doc_id: DocumentID | null
    //hb_ctx?: HeartbeatCtx,
    //start_time_msec: number,
    local_addr: string
    local_port: number
    state: PeerState
    //peer_collection?: Collection,
    //peer_consumer?: PeerConsumer,

    constructor(id: PeerId, ditto: Ditto, coord_addr: string, coord_port: number,
                bind_addr: string, bind_port: number) {
        this.id = id
        this.ditto = ditto
        this.coord_addr = coord_addr
        this.coord_port = coord_port
        this.coord_doc_id = null
        this.coord_collection = COORD_COLLECTION_NAME
        this.coord_info = null
        this.transport_config = null
        this.hb_doc_id = null
        this.local_addr = bind_addr
        this.local_port = bind_port
        this.state = PeerState.Init
    }

    stateTransition(from: PeerState, to: PeerState): void {
        if (this.state != from) {
            throw new Error(`Invalid state transition from ${from} to ${to}`)
        }
        this.state = to
    }

    toPeer(): Peer {
        return {
            peer_id: this.id,
            peer_ip_addr: this.local_addr,
            peer_port: this.local_port,
            peer_state: this.state
        }
    }
}