import {random_peer_id} from './util.js'

test('random_peer_id', () => {
    const id = random_peer_id("tspeer")
    expect(id).toMatch(new RegExp('tspeer_[0-9a-f]{8}'))
})

