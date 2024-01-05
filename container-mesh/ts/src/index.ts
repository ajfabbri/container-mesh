import { CmeshPeer, CmeshEvent, PeerArgs } from './cmpeer';

// main function
async function main() {
    // TODO from command line
    const pargs: PeerArgs = {
        coord_addr: "127.0.0.1",
        coord_port: 4001,
        device_name: "tspeer",
        bind_addr: "127.0.0.1",
        bind_port: 4010,
        output_dir: "output"
    }
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
}).catch(e => {
    console.error(e);
})
