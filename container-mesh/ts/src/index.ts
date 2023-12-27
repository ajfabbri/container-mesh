import { CmeshPeer, CmeshEvent, PeerArgs } from './cmpeer';

// main function
async function main() {
    // TODO from command line
    let pargs: PeerArgs = {
        coord_addr: "localhost",
        coord_port: 4001,
        peer_name: "ts-peer",
        bind_addr: "localhost",
        bind_port: 4010,
        output_dir: "output"
    }
    let cmp = new CmeshPeer(pargs)
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

try {
    main()
} catch (e) {
    console.log(e);
}
