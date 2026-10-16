const fs = require('node:fs');
const vm = require('node:vm');
const cheerio = require('cheerio');
const path = require('node:path');

function loadSource(request) {
    class Element {
        constructor(selection, select) { this.selection = selection; this.select = select; }
        get text() { return this.selection.text(); }
        get attributes() { return this.selection.attr() || {}; }
        querySelector(selector) {
            const selection = this.selection.find(selector).first();
            return selection.length ? new Element(selection, this.select) : null;
        }
        querySelectorAll(selector) {
            return this.selection.find(selector).toArray().map(element => new Element(this.select(element), this.select));
        }
    }
    class Document {
        constructor(html) { this.select = cheerio.load(html); }
        querySelector(selector) {
            const selection = this.select(selector).first();
            return selection.length ? new Element(selection, this.select) : null;
        }
        querySelectorAll(selector) { return this.select(selector).toArray().map(element => new Element(this.select(element), this.select)); }
        dispose() {}
    }
    const construct = function(values) { Object.assign(this, values); };
    const context = vm.createContext({
        ComicSource: class {}, Comic: construct, ComicDetails: construct, HtmlDocument: Document,
        Network: {get: request},
    });
    const code = fs.readFileSync(path.join(__dirname, '../sources/manhuazhan/venera.js'), 'utf8');
    return vm.runInContext(code + '\nnew Manhuazhan();', context);
}

module.exports = {loadSource};
