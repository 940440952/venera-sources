const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');

const root = path.join(__dirname, '..');

test('catalogs refer to the implemented source with synchronized independent versions', () => {
    const venera = JSON.parse(fs.readFileSync(path.join(root, 'index.json'), 'utf8'));
    const aidoku = JSON.parse(fs.readFileSync(path.join(root, 'aidoku/index.json'), 'utf8'));
    const metadata = JSON.parse(fs.readFileSync(path.join(root, 'sources/manhuazhan/aidoku/res/source.json'), 'utf8')).info;
    const code = fs.readFileSync(path.join(root, 'sources/manhuazhan/venera.js'), 'utf8');
    assert.equal(venera.length, 1);
    assert.equal(venera[0].version, code.match(/\bversion\s*=\s*"([^"]+)"/)[1]);
    assert.equal(venera[0].url, code.match(/\burl\s*=\s*"([^"]+)"/)[1]);
    assert.equal(aidoku.sources[0].id, metadata.id);
    assert.equal(aidoku.sources[0].version, metadata.version);
    assert.equal(aidoku.sources[0].minAppVersion, metadata.minAppVersion);
    assert.ok(aidoku.sources[0].downloadURL.endsWith(`/aidoku-${metadata.id}-v${metadata.version}/${metadata.id}-v${metadata.version}.aix`));
});
