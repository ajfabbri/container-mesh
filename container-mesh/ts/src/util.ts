import { existsSync, rmSync } from 'node:fs'
import { Ditto, IdentityOfflinePlayground } from "@dittolive/ditto"

// TODO camelCase functions
export function random_peer_id(peer_name: string): string {
    // prefix + hexidecimal random u64
    return `${peer_name}_${Math.random().toString(16).substr(2, 8)}`
}

export function make_ditto(appId?: string): Ditto {
    // get identity from DITTO_APP_ID env var
    if (appId === undefined) {
        appId = process.env.DITTO_APP_ID
    }
    if (appId === undefined) {
        throw new Error("DITTO_APP_ID env var not set")
    }
    const identity: IdentityOfflinePlayground = {
        appID: appId,
        type: "offlinePlayground"
    }
    // TODO make configurable
    const randStr = Math.random().toString(16).substr(2, 8)
    const persist_dir = `/tmp/ditto-${randStr}`
    // Remove existing persisted data
    if (existsSync(persist_dir)) {
        console.log(`Removing existing Ditto data in ${persist_dir}`)
        rmSync(persist_dir, {recursive: true})
    }
    return new Ditto(identity, persist_dir)
}

export function system_time_msec(): number {
    return Date.now()
}
