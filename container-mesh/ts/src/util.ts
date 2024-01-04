import { existsSync, rm } from 'node:fs'
import { Ditto, IdentityOfflinePlayground } from "@dittolive/ditto"

export function random_peer_id(peer_name: string): string {
    // prefix + hexidecimal random u64
    return `${peer_name}_${Math.random().toString(16).substr(2, 8)}`
}

export function make_ditto(): Ditto {
    // get identity from DITTO_APP_ID env var
    const app_id = process.env.DITTO_APP_ID
    if (!app_id) {
        throw new Error("DITTO_APP_ID env var not set")
    }
    const identity: IdentityOfflinePlayground = {
        appID: app_id,
        type: "offlinePlayground"
    }
    // TODO make configurable
    const persist_dir = "/tmp/ditto"
    // Remove existing persisted data
    if (existsSync(persist_dir)) {
        console.log(`Removing existing Ditto data in ${persist_dir}`)
        rm(persist_dir, {recursive: true})
    }
    return new Ditto(identity, persist_dir)
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function stringify(obj: any): string {
    return JSON.stringify(obj, (_k,v) => {
        typeof v === "bigint" ? v.toString() : v
    })
}
