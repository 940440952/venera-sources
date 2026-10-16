/// Extract a displayed chapter number without including any subtitle.
pub fn chapter_number(title: &str) -> Option<f32> {
    let rest = title.trim_start().strip_prefix('第')?;
    let end = rest.find(['话', '回', '章', '卷'])?;
    let number = rest[..end].trim();
    number
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .or_else(|| chinese_number(number).map(|value| value as f32))
}

fn chinese_number(number: &str) -> Option<u32> {
    if number.is_empty() {
        return None;
    }
    let mut total = 0u32;
    let mut section = 0u32;
    let mut digits = 0u32;
    let mut previous_unit = 10_000;
    let mut has_wan = false;
    for character in number.chars() {
        let digit = match character {
            '零' | '〇' => Some(0),
            '一' => Some(1),
            '二' | '两' => Some(2),
            '三' => Some(3),
            '四' => Some(4),
            '五' => Some(5),
            '六' => Some(6),
            '七' => Some(7),
            '八' => Some(8),
            '九' => Some(9),
            _ => None,
        };
        if let Some(digit) = digit {
            digits = digits.checked_mul(10)?.checked_add(digit)?;
            continue;
        }
        let unit = match character {
            '十' => 10,
            '百' => 100,
            '千' => 1_000,
            '万' if !has_wan => {
                section = section.checked_add(digits)?;
                total = section.max(1).checked_mul(10_000)?;
                section = 0;
                digits = 0;
                previous_unit = 10_000;
                has_wan = true;
                continue;
            }
            _ => return None,
        };
        if unit >= previous_unit {
            return None;
        }
        section = section.checked_add(digits.max(1).checked_mul(unit)?)?;
        digits = 0;
        previous_unit = unit;
    }
    total.checked_add(section)?.checked_add(digits)
}

/// The site ID is only an ordering hint, never a displayed chapter number.
pub fn order_hint(chapter_number: Option<f32>, key: &str) -> Option<f32> {
    chapter_number.or_else(|| {
        let (_, value) = key.rsplit_once('-')?;
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse::<f32>().ok().filter(|value| value.is_finite())
    })
}

/// Aidoku expects newest -> oldest, regardless of the website's directory order.
pub fn ensure_newest_first<T>(chapters: &mut [T], hint: impl Fn(&T) -> Option<f32>) {
    if chapters
        .first()
        .and_then(&hint)
        .zip(chapters.last().and_then(&hint))
        .is_some_and(|(first, last)| first < last)
    {
        chapters.reverse();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chapter_numbers_and_subtitles() {
        assert_eq!(chapter_number("第16话"), Some(16.0));
        assert_eq!(chapter_number("第16回 三川三真 上"), Some(16.0));
        assert_eq!(chapter_number("第十六回 三川三真 上"), Some(16.0));
        assert_eq!(chapter_number("第十六话"), Some(16.0));
        assert_eq!(chapter_number("第一百零二话"), Some(102.0));
        assert_eq!(chapter_number(" 第 16.5 章 番外"), Some(16.5));
        assert_eq!(chapter_number("第两卷"), Some(2.0));
    }

    #[test]
    fn parses_chinese_digits_and_units() {
        for (number, expected) in [
            ("零", 0.0),
            ("〇", 0.0),
            ("十", 10.0),
            ("十一", 11.0),
            ("二十", 20.0),
            ("九十九", 99.0),
            ("两百〇二", 202.0),
            ("两千零二十", 2020.0),
            ("二〇二四", 2024.0),
            ("一万零一", 10001.0),
            ("十二万三千四百五十六", 123456.0),
        ] {
            let title = alloc::format!("第{number}回");
            assert_eq!(chapter_number(&title), Some(expected), "{title}");
        }
    }

    #[test]
    fn rejects_missing_invalid_and_overflowing_numbers() {
        for title in [
            "番外篇",
            "第话",
            "第未知回",
            "十六回",
            "第16",
            "第NaN话",
            "第inf回",
            "第-1章",
            "第十百话",
            "第一万万话",
            "第一亿话",
            "第九九九九九九九九九九九九回",
        ] {
            assert_eq!(chapter_number(title), None, "{title}");
        }
    }

    #[test]
    fn reverses_oldest_first_and_preserves_newest_first() {
        let mut chapters = ["第十五回", "第十六回", "第十七回"];
        ensure_newest_first(&mut chapters, |title| chapter_number(title));
        assert_eq!(
            chapters.map(chapter_number),
            [Some(17.0), Some(16.0), Some(15.0)]
        );
        ensure_newest_first(&mut chapters, |title| chapter_number(title));
        assert_eq!(chapters, ["第十七回", "第十六回", "第十五回"]);
    }

    #[test]
    fn falls_back_to_keys_without_changing_displayed_numbers() {
        let mut chapters = [
            (None, "12345-100"),
            (None, "12345-101"),
            (None, "12345-102"),
        ];
        ensure_newest_first(&mut chapters, |(number, key)| order_hint(*number, key));
        assert_eq!(
            chapters,
            [
                (None, "12345-102"),
                (None, "12345-101"),
                (None, "12345-100")
            ]
        );
        ensure_newest_first(&mut chapters, |(number, key)| order_hint(*number, key));
        assert_eq!(chapters[0], (None, "12345-102"));
        assert_eq!(order_hint(Some(16.0), "12345-100"), Some(16.0));
        assert_eq!(order_hint(None, "prefix-12345-100"), Some(100.0));
        for key in ["12345", "12345-", "12345-unknown", "12345-NaN", "12345-inf"] {
            assert_eq!(order_hint(None, key), None);
        }
    }

    #[test]
    fn preserves_empty_single_equal_and_unknown_orders() {
        let mut empty: [Option<f32>; 0] = [];
        ensure_newest_first(&mut empty, |number| *number);
        let mut single = [Some(16.0)];
        ensure_newest_first(&mut single, |number| *number);
        assert_eq!(single, [Some(16.0)]);
        for original in [
            [Some(16.0), Some(16.0)],
            [None, Some(16.0)],
            [Some(16.0), None],
        ] {
            let mut chapters = original;
            ensure_newest_first(&mut chapters, |number| *number);
            assert_eq!(chapters, original);
        }
    }
}
