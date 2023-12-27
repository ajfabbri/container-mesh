import { CmeshPeer, CmeshEvent, PeerArgs, defaultPeerArgs } from './cmpeer';

test('cmditto app lifecycle', async () => {
    let begin = false
    let end = false
    let exit = false
    let pargs: PeerArgs = defaultPeerArgs
    let cmpeer = new CmeshPeer(pargs)
    let fut = cmpeer.start(async (event: CmeshEvent) => {
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
})
