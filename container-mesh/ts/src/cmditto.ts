import { Ditto } from '@dittolive/ditto';

export class CmeshDitto {
    ditto: Ditto;
    hello(who: string): void {
        console.log(`Hello ${who}!`);
    }
    constructor() {
        this.ditto = new Ditto();
    }
}
