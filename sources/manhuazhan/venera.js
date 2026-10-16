class Manhuazhan extends ComicSource {
    name = "漫画站";
    key = "manhuazhan";
    version = "1.0.0";
    minAppVersion = "1.6.0";
    url = "https://cdn.jsdelivr.net/gh/940440952/venera-sources@main/sources/manhuazhan/venera.js";
    baseUrl = "https://m.manhuazhan.com";
    keys = ["4-bXd9iN", "4-RXyjry", "4-oYvwVy", "4-4ZY57U", "4-mbJpU7", "4-6MM2Ei", "4-54TiQr", "4-Ph5xx9", "4-bYgePR", "4-Z9A3bW"];

    absolute(value) {
        const address = (value || "").trim();
        if (!address || /^(data|javascript):/i.test(address)) return "";
        if (address.startsWith("//")) return "https:" + address;
        if (/^https?:\/\//i.test(address)) return address.replace(/^http:/i, "https:");
        return this.baseUrl + (address.startsWith("/") ? address : "/" + address);
    }

    async request(path) {
        const response = await Network.get(this.absolute(path), {Referer: this.baseUrl + "/"});
        if (response.status !== 200) throw `漫画站请求失败：HTTP ${response.status}`;
        if (/验证码|人机验证|Just a moment|cf-chl-/i.test(response.body)) {
            throw "漫画站要求网页验证，请在浏览器完成验证后重试；可先从首页或分类浏览。";
        }
        return response.body;
    }

    text(root, selector) {
        return (root.querySelector(selector)?.text || "").trim();
    }

    meta(root, property) {
        return root.querySelector(`meta[property="${property}"]`)?.attributes.content || "";
    }

    image(element) {
        if (!element) return "";
        const attributes = element.attributes;
        return this.absolute(attributes["data-original"] || attributes["data-src"] || attributes["data-lazy-src"] || attributes.src);
    }

    parseList(html) {
        const document = new HtmlDocument(html);
        try {
            const comics = [];
            const seen = new Set();
            for (const card of document.querySelectorAll(".comic_cover_container, .home_recommend_comic, .search_list")) {
                const anchor = card.querySelector('a[href*="/comic/"]');
                const match = anchor?.attributes.href?.match(/\/comic\/(\d+)/);
                if (!match || seen.has(match[1])) continue;
                const title = this.text(card, ".comic_cover_title, .sarchList_title") || anchor.attributes.title || "";
                const cover = this.image(card.querySelector(".comic_cover, img"));
                if (!title || !cover) continue;
                seen.add(match[1]);
                comics.push(new Comic({id: match[1], title, cover, subTitle: this.text(card, ".comic_cover_desc, .sina_nickname")}));
            }
            const hasNext = document.querySelectorAll("a.page, .pagination a, a[rel=next]").some(anchor => /下一页|next/i.test(anchor.text) || anchor.attributes.rel === "next");
            return {comics, hasNext};
        } finally {
            document.dispose();
        }
    }

    async listing(path, page) {
        const current = Math.max(1, page || 1);
        const pagedPath = current === 1 ? path : path.replace(/\/$/, "") + "/page/" + current;
        const result = this.parseList(await this.request(pagedPath));
        return {comics: result.comics, maxPage: result.hasNext ? current + 1 : current};
    }

    explore = [{title: "漫画站首页", type: "multiPageComicList", load: async page => {
        if ((page || 1) > 1) return {comics: [], maxPage: 1};
        return {comics: this.parseList(await this.request("/")).comics, maxPage: 1};
    }}];

    category = {
        title: "漫画站分类",
        parts: [{name: "地区", type: "fixed", categories: [
            {label: "全部", target: {page: "category", attributes: {category: "all"}}},
            {label: "国产漫画", target: {page: "category", attributes: {category: "1"}}},
            {label: "日本漫画", target: {page: "category", attributes: {category: "2"}}},
            {label: "韩国漫画", target: {page: "category", attributes: {category: "3"}}},
            {label: "欧美漫画", target: {page: "category", attributes: {category: "4"}}}
        ]}]
    };

    categoryComics = {load: async (category, param, options, page) => {
        if (!["all", "1", "2", "3", "4"].includes(category)) throw "未知分类";
        return this.listing(category === "all" ? "/category" : "/category/list/" + category, page);
    }};

    search = {load: async (keyword, options, page) => {
        const current = Math.max(1, page || 1);
        const query = keyword.trim();
        if (/^\d+$/.test(query)) {
            if (current > 1) return {comics: [], maxPage: 1};
            const details = await this.comic.loadInfo(query);
            return {comics: [new Comic({id: query, title: details.title, cover: details.cover})], maxPage: 1};
        }
        const result = this.parseList(await this.request("/index.php/search?key=" + encodeURIComponent(query) + "&page=" + current));
        return {comics: result.comics, maxPage: result.hasNext ? current + 1 : current};
    }};

    decodeBase64(value) {
        const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bits = 0;
        let bitCount = 0;
        let result = "";
        if (!/^[A-Za-z0-9+/]*={0,2}$/.test(value) || value.length % 4 === 1) throw "图片数据格式无效";
        for (const character of value.replace(/=+$/, "")) {
            bits = (bits << 6) | alphabet.indexOf(character);
            bitCount += 6;
            if (bitCount >= 8) {
                bitCount -= 8;
                result += String.fromCharCode((bits >> bitCount) & 255);
            }
        }
        return result;
    }

    parseImages(html) {
        const document = new HtmlDocument(html);
        let images = [];
        try {
            images = document.querySelectorAll("#ChapterContent p.chapterpic img, .chapter_content img").map(element => this.image(element)).filter(address => address && !/lazyload|loading\.(gif|png)/i.test(address));
        } finally {
            document.dispose();
        }
        if (images.length === 0) {
            const payload = html.match(/\bvar\s+DATA\s*=\s*['"]([^'"]+)['"]/);
            const chapterId = html.match(/\bvar\s+cid\s*=\s*(\d+)/);
            const legacy = html.match(/\b(?:var\s+)?newImgs\s*=\s*(\[[\s\S]*?\])\s*;/);
            let entries;
            if (payload && chapterId) {
                const key = this.keys[Number(chapterId[1]) % 10];
                const encrypted = this.decodeBase64(payload[1]);
                let inner = "";
                for (let position = 0; position < encrypted.length; position++) {
                    inner += String.fromCharCode(encrypted.charCodeAt(position) ^ key.charCodeAt(position % key.length));
                }
                entries = JSON.parse(this.decodeBase64(inner));
            } else if (legacy) {
                entries = JSON.parse(legacy[1]);
            }
            if (Array.isArray(entries)) images = entries.map(entry => this.absolute(typeof entry === "string" ? entry : entry.url)).filter(Boolean);
        }
        if (!images.length) throw "没有解析到章节图片，站点格式可能已变化。";
        return [...new Set(images)];
    }

    comic = {
        loadInfo: async id => {
            if (!/^\d+$/.test(id)) throw "无效漫画 ID";
            const document = new HtmlDocument(await this.request("/comic/" + id));
            try {
                const chapters = {};
                for (const anchor of document.querySelectorAll('.catalog_list a[href*="/chapter/"], .d-play-list a[href*="/chapter/"], .d-player-list a[href*="/chapter/"]')) {
                    const match = anchor.attributes.href?.match(/\/chapter\/(\d+-\d+)\.html/);
                    if (match && match[1].startsWith(id + "-") && !chapters[match[1]]) chapters[match[1]] = this.text(anchor, ".name") || anchor.text.trim();
                }
                const title = this.meta(document, "og:title") || this.text(document, ".comic_name h1, h1");
                if (!title || !Object.keys(chapters).length) throw "漫画详情或章节目录为空，站点格式可能已变化。";
                const author = this.meta(document, "og:author") || this.text(document, ".author_name");
                return new ComicDetails({
                    title,
                    cover: this.absolute(this.meta(document, "og:image")) || this.image(document.querySelector(".comic_info .comic_cover")),
                    description: this.meta(document, "og:description") || this.text(document, ".details"),
                    subTitle: author,
                    tags: {作者: author ? [author] : [], 标签: this.text(document, ".comic_tags").split(/\s+/).filter(Boolean)},
                    chapters,
                    url: this.baseUrl + "/comic/" + id
                });
            } finally {
                document.dispose();
            }
        },
        loadEp: async (comicId, epId) => {
            if (!/^\d+-\d+$/.test(epId || "") || !epId.startsWith(comicId + "-")) throw "无效章节 ID";
            return {images: this.parseImages(await this.request("/chapter/" + epId + ".html"))};
        },
        onImageLoad: () => ({headers: {Referer: this.baseUrl + "/"}}),
        onThumbnailLoad: () => ({headers: {Referer: this.baseUrl + "/"}}),
        link: {domains: ["m.manhuazhan.com", "www.manhuazhan.com", "manhuazhan.com"], linkToId: address => address.match(/\/comic\/(\d+)/)?.[1] || null}
    };
}
