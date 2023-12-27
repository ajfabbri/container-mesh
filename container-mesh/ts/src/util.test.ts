import {random_peer_id} from './util'

test('random_peer_id', () => {
    let id = random_peer_id("ts-peer")
    expect(id).toMatch(new RegExp('ts-peer_[0-9a-f]{8}'))
})

