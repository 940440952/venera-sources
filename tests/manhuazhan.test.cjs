const assert = require('node:assert/strict');
const test = require('node:test');
const fixture = require('./fixtures/manhuazhan/encrypted.json');
const {loadSource} = require('./runtime.cjs');

const card = '<div class="comic_cover_container"><a href="/comic/219430"><div class="comic_cover" data-original="//images.example/cover.jpg"></div></a><div class="comic_cover_title"><a href="/comic/219430">斗罗大陆</a></div></div>';
const details = '<meta property="og:title" content="斗罗大陆"><meta property="og:author" content="风炫动漫"><meta property="og:image" content="//images.example/cover.jpg"><meta property="og:description" content="作品简介"><ul class="catalog_list"><a href="/chapter/219430-1.html"><p class="name">第1话</p></a><a href="/chapter/219430-2.html"><p class="name">第2话</p></a><a href="/chapter/2-1.html">其他漫画</a></ul>';

test('lists deduplicate cards, preserve Unicode titles, and handle real next-page links', async () => {
    const addresses = [];
    const source = loadSource(async address => {
        addresses.push(address);
        return {status: 200, body: card + card + '<a class="page" href="/category/list/1/page/3">下一页</a>'};
    });
    const result = await source.categoryComics.load('1', null, [], 2);
    assert.equal(addresses[0], 'https://m.manhuazhan.com/category/list/1/page/2');
    assert.equal(result.comics.length, 1);
    assert.equal(result.comics[0].title, '斗罗大陆');
    assert.equal(result.comics[0].cover, 'https://images.example/cover.jpg');
    assert.equal(result.maxPage, 3);
});

test('loads details and numeric ID searches without requiring site keyword search', async () => {
    const source = loadSource(async () => ({status: 200, body: details}));
    const result = await source.comic.loadInfo('219430');
    assert.deepEqual(Object.keys(result.chapters), ['219430-1', '219430-2']);
    assert.equal(result.title, '斗罗大陆');
    assert.equal(result.subTitle, '风炫动漫');
    assert.equal((await source.search.load('219430', [], 1)).comics[0].id, '219430');
});

test('decodes the current live encrypted payload, preserving page order', () => {
    const source = loadSource();
    const images = source.parseImages(`var cid=${fixture.cid};var DATA='${fixture.data}';`);
    assert.equal(images.length, fixture.count);
    assert.equal(images[0], fixture.first);
});

test('all ten chapter key rotations work, without evaluating website JavaScript', () => {
    const source = loadSource();
    const inner = Buffer.from(JSON.stringify([{url: 'https://example.org/1.jpg'}])).toString('base64');
    source.keys.forEach((key, suffix) => {
        const encrypted = Buffer.from([...inner].map((character, position) => character.charCodeAt(0) ^ key.charCodeAt(position % key.length))).toString('base64');
        assert.equal(source.parseImages(`var cid=${suffix};var DATA='${encrypted}';`)[0], 'https://example.org/1.jpg');
    });
});

test('supports legacy JSON image arrays and current HTML lazy image nodes', () => {
    const source = loadSource();
    assert.equal(source.parseImages('var newImgs=["//example.org/legacy.jpg"];')[0], 'https://example.org/legacy.jpg');
    assert.equal(source.parseImages('<div id="ChapterContent"><p class="chapterpic"><img src="loading.gif" data-src="https://example.org/current.jpg"></p></div>')[0], 'https://example.org/current.jpg');
    assert.throws(() => source.parseImages('<html>empty</html>'), /没有解析到/);
});

test('reports verification and HTTP failures, rejects cross-comic chapter IDs', async () => {
    const blocked = loadSource(async () => ({status: 200, body: '请输入验证码'}));
    await assert.rejects(blocked.search.load('斗罗&大陆', [], 1), error => String(error).includes('网页验证'));
    const failed = loadSource(async () => ({status: 503, body: ''}));
    await assert.rejects(failed.request('/'), error => String(error).includes('503'));
    await assert.rejects(failed.comic.loadEp('219430', '2-3'), error => String(error).includes('无效章节'));
    assert.equal(failed.comic.onImageLoad().headers.Referer, 'https://m.manhuazhan.com/');
});
