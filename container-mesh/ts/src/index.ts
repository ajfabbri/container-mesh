import { exit } from 'node:process';
import { CmeshPeer, CmeshEvent } from './cmpeer';
import { PeerArgs, parseCLIArgs } from './peerargs';

// main function
async function main() {
    const pargs: PeerArgs = parseCLIArgs()
    const cmp = new CmeshPeer(pargs)
    await cmp.start(async (event: CmeshEvent) => {
        switch (event) {
            case CmeshEvent.BeginTest:
                console.log("BeginTest")
                break
            case CmeshEvent.EndTest:
                console.log("EndTest")
                break
            case CmeshEvent.Exiting:
                console.log("Exiting")
                break
            default:
                // fail assertion in this case
                fail("Unexpected event")
            }
        })
}

main().then(() => {
    console.debug("Done")
    exit(0)
}).catch(e => {
    console.error(e);
})
