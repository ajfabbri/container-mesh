import { Ditto, DittoError, IdentityOfflinePlayground } from "@dittolive/ditto"

export function random_peer_id(peer_name: string): string {
    // prefix + hexidecimal random u64
    return `${peer_name}_${Math.random().toString(16).substr(2, 8)}`
}

export function make_ditto(): Ditto {
    // get identity from DITTO_APP_ID env var
    let app_id = process.env.DITTO_APP_ID
    if (!app_id) {
        throw new Error("DITTO_APP_ID env var not set")
    }
    let identity: IdentityOfflinePlayground = {
        appID: app_id,
        type: "offlinePlayground"
    }
    // TODO make configurable
    let persist_dir = "/tmp/ditto"
    return new Ditto(identity, persist_dir)
}
