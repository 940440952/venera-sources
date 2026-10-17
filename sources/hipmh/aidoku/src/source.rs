use crate::{
    decoder,
    parser::{self, API_URL, BASE_URL, READER_URL},
};
use aidoku::{
    AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
    Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
    Result, Source,
    alloc::{String, Vec, format},
    imports::{html::Html, net::Request},
    prelude::*,
};
use serde_json::Value;

struct Hipmh;

fn text(value: &Value, field: &str) -> Option<String> {
    value[field]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(String::from)
}

fn names(value: &Value, field: &str) -> Option<Vec<String>> {
    value[field].as_array().map(|values| {
        values
            .iter()
            .filter_map(|value| {
                value
                    .as_str()
                    .map(String::from)
                    .or_else(|| text(value, "name"))
            })
            .collect()
    })
}

impl Hipmh {
    fn get(url: &str) -> Result<String> {
        let response = Request::get(url)?
            .header("Referer", &format!("{BASE_URL}/"))
            .send()?;
        if response.status_code() != 200 {
            bail!("Hipmh 请求失败：HTTP {}", response.status_code());
        }
        response.get_string()
    }

    fn api(url: &str) -> Result<Value> {
        let value: Value = serde_json::from_str(&Self::get(url)?)
            .map_err(|_| AidokuError::message("Hipmh API 返回无效 JSON"))?;
        if value["code"].as_i64() != Some(200) {
            bail!(
                "Hipmh API 错误：{}",
                value["message"].as_str().unwrap_or("未知错误")
            );
        }
        if !value["data"].is_object() {
            bail!("Hipmh API 缺少数据");
        }
        Ok(value["data"].clone())
    }

    fn manga(value: &Value, search: bool) -> Option<Manga> {
        let key = text(value, if search { "id" } else { "mid" })?;
        if !parser::valid_key(&key) {
            return None;
        }
        let cover = text(value, "vertical_image_url")
            .or_else(|| text(value, "cover_image_url"))
            .and_then(|path| parser::absolute_image(&path, "https://cover.s3imgs.top"));
        Some(Manga {
            url: Some(format!("{BASE_URL}/works/{key}")),
            key,
            title: text(value, "title")?,
            cover,
            authors: names(value, "authors").or_else(|| names(value, "author_names")),
            tags: names(value, "genres"),
            description: text(value, "description"),
            status: match value["status"].as_str() {
                Some("ongoing") => MangaStatus::Ongoing,
                Some("completed") => MangaStatus::Completed,
                _ => MangaStatus::Unknown,
            },
            ..Default::default()
        })
    }

    fn list(page: i32, sort: &str, query: Option<&str>) -> Result<MangaPageResult> {
        let page = page.max(1);
        let search = query.is_some();
        let url = if let Some(query) = query {
            format!(
                "{API_URL}/v1/search?q={}&page={page}&page_size=20",
                parser::urlencode(query)
            )
        } else {
            format!("{API_URL}/v1/mangas?sort={sort}&page={page}&per_page=20")
        };
        let data = Self::api(&url)?;
        let items = data[if search { "data" } else { "items" }]
            .as_array()
            .ok_or_else(|| AidokuError::message("Hipmh 列表数据缺失"))?;
        Ok(MangaPageResult {
            entries: items
                .iter()
                .filter_map(|item| Self::manga(item, search))
                .collect(),
            has_next_page: i64::from(page) < data["total_pages"].as_i64().unwrap_or(1),
        })
    }
}

impl Source for Hipmh {
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
        Self::list(page, "updated", (!query.is_empty()).then_some(query))
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        if !parser::valid_key(&manga.key) {
            bail!("无效漫画 ID");
        }
        let mid = manga.key.split('-').next().unwrap_or(&manga.key);
        let mid = parser::urlencode(mid);
        if needs_details {
            let mut data = Self::api(&format!("{API_URL}/v1/manga?mid={mid}"))?;
            data["mid"] = Value::String(manga.key.clone());
            let details = Self::manga(&data, false)
                .ok_or_else(|| AidokuError::message("Hipmh 漫画详情缺失"))?;
            manga.title = details.title;
            manga.cover = details.cover;
            manga.authors = details.authors;
            manga.tags = details.tags;
            manga.description = details.description;
            manga.status = details.status;
            manga.url = details.url;
        }
        if needs_chapters {
            let mut chapters: Vec<Chapter> = Vec::new();
            let mut page = 1;
            loop {
                let data = Self::api(&format!(
                    "{API_URL}/v1/manga/chapters?mid={mid}&page={page}&per_page=50&order=desc"
                ))?;
                let items = data["items"]
                    .as_array()
                    .ok_or_else(|| AidokuError::message("Hipmh 章节数据缺失"))?;
                for item in items {
                    let key = text(item, "hid")
                        .filter(|key| parser::valid_key(key))
                        .ok_or_else(|| AidokuError::message("Hipmh 章节 ID 无效"))?;
                    if chapters.iter().any(|chapter| chapter.key == key) {
                        continue;
                    }
                    let chapter_number = item["chapter_number"]
                        .as_f64()
                        .filter(|number| *number >= 0.0)
                        .map(|number| number as f32);
                    let title = text(item, "title")
                        .or_else(|| chapter_number.map(|number| format!("第{number}话")));
                    let date_uploaded = text(item, "updated_at")
                        .and_then(|date| {
                            date.get(..10).and_then(|date| {
                                aidoku::imports::std::parse_date(date, "yyyy-MM-dd")
                            })
                        })
                        .map(|timestamp| timestamp as i64);
                    chapters.push(Chapter {
                        url: Some(format!("{READER_URL}/chapter/{key}")),
                        key,
                        title,
                        chapter_number,
                        date_uploaded,
                        language: Some("zh".into()),
                        ..Default::default()
                    });
                }
                if page >= data["total_pages"].as_i64().unwrap_or(1) {
                    break;
                }
                if items.is_empty() || page >= 10000 {
                    bail!("Hipmh 章节分页异常");
                }
                page += 1;
            }
            if chapters.is_empty() {
                bail!("Hipmh 章节目录为空");
            }
            manga.chapters = Some(chapters);
        }
        Ok(manga)
    }

    fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        if !parser::valid_key(&chapter.key) {
            bail!("无效章节 ID");
        }
        let reader_url = format!("{READER_URL}/chapter/{}", chapter.key);
        let document = Html::parse_with_url(Self::get(&reader_url)?, &reader_url)?;
        let config = document
            .select_first("#chapcontent")
            .ok_or_else(|| AidokuError::message("Hipmh 阅读配置缺失"))?;
        let api_hid = config
            .attr("data-api-hid")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AidokuError::message("Hipmh 章节 API ID 缺失"))?;
        let base = config
            .attr("data-api-base-url")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| API_URL.into());
        let api_url = parser::chapter_api(&base)
            .ok_or_else(|| AidokuError::message("Hipmh 章节 API 地址无效"))?;
        let data = Self::api(&format!("{api_url}?hid={}", parser::urlencode(&api_hid)))?;
        let attributes = if data["line"].as_i64() == Some(9) {
            [
                "data-chapter-img-base-line1s",
                "data-chapter-img-base-line2s",
                "data-chapter-img-base",
            ]
        } else {
            [
                "data-chapter-img-base-line1",
                "data-chapter-img-base-line2",
                "data-chapter-img-base",
            ]
        };
        let host = attributes
            .iter()
            .find_map(|attribute| config.attr(attribute).filter(|value| !value.is_empty()))
            .ok_or_else(|| AidokuError::message("Hipmh 图片域名缺失"))?;
        let mut images = decoder::decode(
            data["images"]
                .as_str()
                .ok_or_else(|| AidokuError::message("Hipmh 图片数据缺失"))?,
        )
        .map_err(AidokuError::message)?;
        decoder::remove_decoy(&mut images, data["order_id"].as_i64(), data["sid"].as_i64());
        if images.is_empty() {
            bail!("Hipmh 章节图片为空");
        }
        images
            .iter()
            .map(|path| {
                let url = parser::absolute_image(path, &host)
                    .ok_or_else(|| AidokuError::message("Hipmh 图片地址无效"))?;
                Ok(Page {
                    content: PageContent::url(url),
                    ..Default::default()
                })
            })
            .collect()
    }
}

impl ListingProvider for Hipmh {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        match listing.id.as_str() {
            "updated" | "popular" => Self::list(page, &listing.id, None),
            _ => bail!("未知分类"),
        }
    }
}

impl ImageRequestProvider for Hipmh {
    fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
        Ok(Request::get(url)?.header("Referer", &format!("{READER_URL}/")))
    }
}

impl DeepLinkHandler for Hipmh {
    fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
        if let Some(key) = parser::manga_key(&url) {
            return Ok(Some(DeepLinkResult::Manga { key }));
        }
        if let Some(key) = url.strip_prefix(&format!("{READER_URL}/chapter/")) {
            let key = key.split(['?', '#']).next().unwrap_or_default();
            if !parser::valid_key(key) {
                return Ok(None);
            }
            let document = Html::parse(Self::get(&format!("{READER_URL}/chapter/{key}"))?)?;
            let manga_key = document
                .select_first("#chapcontent")
                .and_then(|config| config.attr("data-manga-path"))
                .and_then(|path| parser::manga_key(&path));
            return Ok(manga_key.map(|key| DeepLinkResult::Manga { key }));
        }
        Ok(None)
    }
}

register_source!(
    Hipmh,
    ListingProvider,
    ImageRequestProvider,
    DeepLinkHandler
);
