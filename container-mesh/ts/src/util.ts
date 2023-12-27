// in typescript
export function random_peer_id(peer_name: string): string {
    // prefix + hexidecimal random u64
    return `${peer_name}_${Math.random().toString(16).substr(2, 8)}`
}
