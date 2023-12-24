import { CmeshDitto, CmeshEvent } from './cmditto';

// main function
async function main() {
    let cmditto = new CmeshDitto()
    await cmditto.start(async (event: CmeshEvent) => {
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
