import fs from "fs";
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/*
 * Prepare dist directory for packaging.
 * We don't want the resulting package to have a dist/ package within it.
 * We can also strip things out of our package.json that aren't needed by
 * consumers of our library.
 *  Thanks to: https://stackoverflow.com/a/52177090
 */
function main() {
    const source = fs.readFileSync(__dirname + "/../package.json").toString('utf-8')
    const sourceObj = JSON.parse(source)
    sourceObj.scripts = {}
    sourceObj.devDependencies = {}
    if (sourceObj.main.startsWith("dist/")) {
        sourceObj.main = sourceObj.main.slice(5);
    }
    const files = sourceObj.files
    for (let i = 0; i < files.length; i++) {
        if (files[i].startsWith("dist/")) {
            files[i] = files[i].slice(5);
        }
    }
    fs.writeFileSync(__dirname + "/package.json", Buffer.from(JSON.stringify(sourceObj, null, 2), "utf-8") )
    fs.copyFileSync(__dirname + "/../.npmignore", __dirname + "/.npmignore")
}

main()
