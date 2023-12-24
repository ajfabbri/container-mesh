import { CmeshDitto, CmeshEvent } from './cmditto';

test('cmditto app lifecycle', async () => {
    let begin = false
    let end = false
    let exit = false
    let cditto = new CmeshDitto()
    let fut = cditto.start(async (event: CmeshEvent) => {
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
