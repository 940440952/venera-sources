use crate::parser::{self, BASE_URL};
use aidoku::{
    AidokuError, Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue,
    ImageRequestProvider, Listing, ListingProvider, Manga, MangaPageResult, Page, PageContent,
    PageContext, Result, Source,
    alloc::{String, Vec, format, vec},
    imports::{
        html::{Document, Element, Html},
        net::Request,
    },
    prelude::*,
};

struct Manhuazhan;

impl Manhuazhan {
    fn request(path: &str) -> Result<(String, Document)> {
        let url = parser::absolute(path).ok_or_else(|| AidokuError::message("无效网址"))?;
        let response = Request::get(&url)?
            .header("Referer", &format!("{BASE_URL}/"))
            .send()?;
        if response.status_code() != 200 {
            bail!("漫画站请求失败：HTTP {}", response.status_code());
        }
        let html = response.get_string()?;
        if ["验证码", "人机验证", "Just a moment", "cf-chl-"]
            .iter()
            .any(|marker| html.contains(marker))
        {
            bail!("漫画站要求网页验证，请在源网页完成验证后重试；可先从首页或分类浏览。");
        }
        let document = Html::parse_with_url(&html, &url)?;
        Ok((html, document))
    }

    fn image(element: Element) -> Option<String> {
        ["data-original", "data-src", "data-lazy-src", "src"]
            .iter()
            .filter_map(|attribute| element.attr(attribute))
            .find_map(|value| parser::absolute(&value))
    }

    fn meta(document: &Document, property: &str) -> Option<String> {
        document
            .select_first(format!("meta[property='{property}']"))?
            .attr("content")
    }

    fn list(path: &str, page: i32) -> Result<MangaPageResult> {
        let path = if page <= 1 {
            String::from(path)
        } else {
            format!("{}/page/{page}", path.trim_end_matches('/'))
        };
        let (_, document) = Self::request(&path)?;
        let mut entries: Vec<Manga> = Vec::new();
        for card in document
            .select(".comic_cover_container, .home_recommend_comic, .search_list")
            .into_iter()
            .flatten()
        {
            let Some(key) = card
                .select_first("a[href*='/comic/']")
                .and_then(|anchor| anchor.attr("href"))
                .and_then(|url| parser::comic_key(&url))
            else {
                continue;
            };
            if entries.iter().any(|entry| entry.key == key) {
                continue;
            }
            let title = card
                .select_first(".comic_cover_title, .sarchList_title")
                .and_then(|element| element.text())
                .unwrap_or_default();
            let cover = card.select_first(".comic_cover, img").and_then(Self::image);
            if title.is_empty() || cover.is_none() {
                continue;
            }
            entries.push(Manga {
                url: Some(format!("{BASE_URL}/comic/{key}")),
                key,
                title,
                cover,
                ..Default::default()
            });
        }
        let has_next_page = document
            .select("a.page, .pagination a, a[rel=next]")
            .into_iter()
            .flatten()
            .any(|anchor| {
                anchor.text().is_some_and(|text| {
                    text.contains("下一页") || text.to_lowercase().contains("next")
                }) || anchor.attr("rel").as_deref() == Some("next")
            });
        Ok(MangaPageResult {
            entries,
            has_next_page,
        })
    }
}

impl Source for Manhuazhan {
    fn new() -> Self {
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        _filters: Vec<FilterValue>,
    ) -> Result<MangaPageResult> {
        let query = query.unwrap_or_default();
        let query = query.trim();
        if query.is_empty() {
            return Self::list("/category", page);
        }
        if query.chars().all(|character| character.is_ascii_digit()) {
            if page > 1 {
                return Ok(MangaPageResult::default());
            }
            let manga = self.get_manga_update(
                Manga {
                    key: query.into(),
                    ..Default::default()
                },
                true,
                false,
            )?;
            return Ok(MangaPageResult {
                entries: vec![manga],
                has_next_page: false,
            });
        }
        let (html, document) = Self::request(&format!(
            "/index.php/search?key={}&page={page}",
            parser::urlencode(query)
        ))?;
        let mut entries = Vec::new();
        for card in document
            .select(".comic_cover_container, .search_list")
            .into_iter()
            .flatten()
        {
            let Some(key) = card
                .select_first("a[href*='/comic/']")
                .and_then(|anchor| anchor.attr("href"))
                .and_then(|url| parser::comic_key(&url))
            else {
                continue;
            };
            if entries.iter().any(|entry: &Manga| entry.key == key) {
                continue;
            }
            let title = card
                .select_first(".comic_cover_title, .sarchList_title")
                .and_then(|element| element.text())
                .unwrap_or_default();
            if !title.is_empty() {
                entries.push(Manga {
                    key,
                    title,
                    cover: card.select_first(".comic_cover, img").and_then(Self::image),
                    ..Default::default()
                });
            }
        }
        Ok(MangaPageResult {
            entries,
            has_next_page: html.contains("下一页"),
        })
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        if manga.key.is_empty()
            || !manga
                .key
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            bail!("无效漫画 ID");
        }
        let (_, document) = Self::request(&format!("/comic/{}", manga.key))?;
        if needs_details {
            manga.title = Self::meta(&document, "og:title")
                .or_else(|| {
                    document
                        .select_first(".comic_name h1, h1")
                        .and_then(|element| element.text())
                })
                .ok_or_else(|| AidokuError::message("漫画标题为空"))?;
            manga.cover = Self::meta(&document, "og:image").and_then(|url| parser::absolute(&url));
            manga.description = Self::meta(&document, "og:description");
            manga.authors = Self::meta(&document, "og:author").map(|author| vec![author]);
            manga.tags = document
                .select_first(".comic_tags")
                .and_then(|element| element.text())
                .map(|tags| tags.split_whitespace().map(String::from).collect());
            manga.url = Some(format!("{BASE_URL}/comic/{}", manga.key));
            manga.content_rating = ContentRating::Unknown;
        }
        if needs_chapters {
            let mut chapters: Vec<Chapter> = Vec::new();
            for anchor in document.select(".catalog_list a[href*='/chapter/'], .d-play-list a[href*='/chapter/'], .d-player-list a[href*='/chapter/']").into_iter().flatten() {
                let Some(key) = anchor.attr("href").and_then(|url| parser::chapter_key(&url, &manga.key)) else { continue; };
                if chapters.iter().any(|chapter| chapter.key == key) { continue; }
                let title = anchor.select_first(".name").and_then(|element| element.text()).or_else(|| anchor.text());
                let chapter_number = title.as_ref().and_then(|title| title.strip_prefix('第')).and_then(|rest| rest.split('话').next()).and_then(|number| number.trim().parse().ok());
                let date_uploaded = anchor.select_first("time").and_then(|element| element.text()).and_then(|date| aidoku::imports::std::parse_date(date, "yyyy-MM-dd")).map(|timestamp| timestamp as i64);
                chapters.push(Chapter {url: Some(format!("{BASE_URL}/chapter/{key}.html")), key, title, chapter_number, date_uploaded, language: Some("zh".into()), ..Default::default()});
            }
            if chapters.is_empty() {
                bail!("章节目录为空，站点格式可能已变化。");
            }
            if chapters.len() > 1
                && chapters
                    .first()
                    .and_then(|chapter| chapter.chapter_number)
                    .zip(chapters.last().and_then(|chapter| chapter.chapter_number))
                    .is_some_and(|(first, last)| first < last)
            {
                chapters.reverse();
            }
            manga.chapters = Some(chapters);
        }
        Ok(manga)
    }

    fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let path = format!("/chapter/{}.html", chapter.key);
        if parser::chapter_key(&path, &manga.key).is_none() {
            bail!("无效章节 ID");
        }
        let (html, document) = Self::request(&path)?;
        let mut images: Vec<String> = document
            .select("#ChapterContent p.chapterpic img, .chapter_content img")
            .into_iter()
            .flatten()
            .filter_map(Self::image)
            .filter(|url| !url.contains("lazyload") && !url.contains("loading.gif"))
            .collect();
        if images.is_empty() {
            images = if html.contains("var DATA") {
                parser::encrypted_images(&html)
            } else if let Some(start) = html.find("newImgs") {
                let tail = &html[start..];
                let begin = tail
                    .find('[')
                    .ok_or_else(|| AidokuError::message("旧版图片数据缺失"))?;
                let end = tail[begin..]
                    .find(']')
                    .ok_or_else(|| AidokuError::message("旧版图片列表不完整"))?
                    + begin;
                parser::json_images(&tail.as_bytes()[begin..=end])
            } else {
                Err("没有解析到章节图片，站点格式可能已变化。")
            }
            .map_err(AidokuError::message)?;
        }
        if images.is_empty() {
            bail!("章节图片为空");
        }
        let mut pages = Vec::new();
        for url in images {
            let mut context = PageContext::new();
            context.insert("referer".into(), format!("{BASE_URL}/"));
            pages.push(Page {
                content: PageContent::url_context(url, context),
                ..Default::default()
            });
        }
        Ok(pages)
    }
}

impl ListingProvider for Manhuazhan {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        match listing.id.as_str() {
            "home" if page > 1 => Ok(MangaPageResult::default()),
            "home" => {
                let mut result = Self::list("/", 1)?;
                result.has_next_page = false;
                Ok(result)
            }
            "all" => Self::list("/category", page),
            "1" | "2" | "3" | "4" => Self::list(&format!("/category/list/{}", listing.id), page),
            _ => bail!("未知分类"),
        }
    }
}

impl ImageRequestProvider for Manhuazhan {
    fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
        Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
    }
}

impl DeepLinkHandler for Manhuazhan {
    fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
        Ok(parser::comic_key(&url).map(|key| DeepLinkResult::Manga { key }))
    }
}

register_source!(
    Manhuazhan,
    ListingProvider,
    ImageRequestProvider,
    DeepLinkHandler
);
