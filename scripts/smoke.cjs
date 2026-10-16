const {execFileSync} = require('node:child_process');
const {loadSource} = require('../tests/runtime.cjs');

async function main() {
    const request = async address => ({status: 200, body: execFileSync('curl', ['-fsSL', '--max-time', '30', '-H', 'Referer: https://m.manhuazhan.com/', address], {encoding: 'utf8', maxBuffer: 4 * 1024 * 1024})});
    const source = loadSource(request);
    const home = await source.explore[0].load(1);
    if (!home.comics.length) throw new Error('Homepage returned no manga');
    const category = await source.categoryComics.load('1', null, [], 1);
    if (!category.comics.length) throw new Error('Category returned no manga');
    const details = await source.comic.loadInfo('219430');
    const chapters = Object.keys(details.chapters);
    if (!chapters.length) throw new Error('Details returned no chapters');
    const chapter = await source.comic.loadEp('219430', chapters[chapters.length - 1]);
    const response = execFileSync('curl', ['-fsSL', '--max-time', '30', '-H', 'Referer: https://m.manhuazhan.com/', chapter.images[0]], {maxBuffer: 12 * 1024 * 1024});
    const isImage = response[0] === 0xff && response[1] === 0xd8 || response.subarray(1, 4).toString() === 'PNG' || response.subarray(0, 4).toString() === 'RIFF';
    if (!isImage) throw new Error('First image did not return image bytes');
    let search;
    try { search = {count: (await source.search.load('斗罗', [], 1)).comics.length}; }
    catch (error) {
        if (!String(error).includes('网页验证')) throw error;
        search = {restricted: true, reason: String(error)};
    }
    console.log(JSON.stringify({home: home.comics.length, category: category.comics.length, title: details.title, chapters: chapters.length, pages: chapter.images.length, imageBytes: response.length, search}, null, 2));
}

main().catch(error => { console.error(error); process.exit(1); });
