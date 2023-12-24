import { Ditto } from '@dittolive/ditto';

export enum CmeshEvent {
    BeginTest,
    EndTest,
    Exiting
};

type CMEventCallback = (event: CmeshEvent) => Promise<void>;
export class CmeshDitto {
    //ditto: Ditto;
    hello(who: string): void {
        console.log(`Hello ${who}!`)
    }
    constructor() {
        //this.ditto = new Ditto()
    }

    // Call start() to get an event callback for state transitions.
    public async start(cb: CMEventCallback) : Promise<string> {
        // XXX
        cb(CmeshEvent.BeginTest)
        // sleep for 1 second
        await new Promise(resolve => setTimeout(resolve, 100));
        await cb(CmeshEvent.EndTest)
        await new Promise(resolve => setTimeout(resolve, 100));
        await cb(CmeshEvent.Exiting)
        return Promise.resolve("OK")
    }

}
