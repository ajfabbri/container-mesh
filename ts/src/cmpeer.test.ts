import { CmeshPeer } from './cmpeer.js'
import { PeerArgs, defaultPeerArgs } from './peerargs.js'

test('cmditto app lifecycle', async () => {
    const pargs: PeerArgs = defaultPeerArgs
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const _cmpeer = new CmeshPeer(pargs)
    /*
     * TODO this initial test no longer works, as the peer needs a coordinator
     * to transition through its lifecycle.
     * Consider:
     * - Adding a proper integration test.
     * - Refactoring this library to allow more unit tests.
     *
    let begin = false
    let end = false
    let exit = false
    const fut = cmpeer.start(async (event: CmeshEvent) => {
        switch (event) {
            case CmeshEvent.BeginTest:
                console.log("BeginTest")
                begin = true
                break
            case CmeshEvent.EndTest:
                console.log("EndTest")
                end = true
                break
            case CmeshEvent.Exiting:
                console.log("Exiting")
                exit = true
                break
            default:
                // fail assertion in this case
                fail("Unexpected event")
        }
    })
    await fut
    expect(begin && end && exit).toBe(true)
    */
})
