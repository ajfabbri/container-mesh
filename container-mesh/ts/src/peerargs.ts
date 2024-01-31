import { parseArgs } from 'node:util'

export interface PeerArgs extends Record<string, string | number | undefined> {
    // coordinator's ip address
    coord_addr: string
    coord_port: number
    device_name: string
    bind_addr: string
    bind_port: number
    output_dir: string
    ditto_license: string | undefined
    ditto_app_id: string | undefined
}

export const defaultPeerArgs: PeerArgs = {
    coord_addr: "127.0.0.1",
    coord_port: 4001,
    device_name: "tspeer",
    bind_addr: "0.0.0.0",
    bind_port: 4010,
    output_dir: "output",
    // Additional args without command-line equivalents
    ditto_license: undefined,
    ditto_app_id: undefined
}

export function parseCLIArgs(): PeerArgs {
    const options = {
        'coord-collection': {
            type: 'string',
            short: 'c',
        },
        'coord-addr': {
            type: 'string',
        },
        'coord-port': {
            type: 'string',
            default: '4001',
        },
        'bind-addr': {
            type: 'string',
            short: 'b',
            default: undefined
        },
        'bind-port': {
            type: 'string',
            short: 'p',
            default: '4010',
        },
        'device-name': {
            type: 'string',
            short: 'd',
            default: 'tspeer',
        },
        'output-dir': {
            type: 'string',
            short: 'o',
            default: '/output',
        }
    } as const // make TS infer tigher types, i.e. string, not any

    // eslint-disable-next-line
    const { values, positionals } = parseArgs({ options, strict: true })
    const pargs = defaultPeerArgs
    console.debug("XXX positionals", positionals)

    for (const [_key, value] of Object.entries(values)) {
        console.debug(`--> CLI arg ${_key}=${value}`)
        const key = _key.replace(/-/g, '_')
        // eslint-disable-next-line no-prototype-builtins
        if (pargs.hasOwnProperty(key)) {
            if (typeof pargs[key] === 'number') {
                pargs[key] = parseInt(value as string)
            } else if (typeof pargs[key] === 'string') {
                pargs[key] = value as string
            }
        } else {
            // shouldn't happen due to strict: true
            console.error(`Ignoring unrecognized option ${key}`)
        }
    }
    return pargs
}
