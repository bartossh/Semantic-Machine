use anyhow::{Result, anyhow};
use regex::bytes::Regex;
use scraper::{Html, Selector};

/// Extracts the article content from a given URL.
///
/// # Arguments
///
/// * `url` - The URL of the article to extract.
///
/// # Returns
///
/// A `Result` containing the extracted article content as a `String`, or an `anyhow::Error` if extraction fails.
pub async fn extract_article(url: &str) -> Result<String> {
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;

    let document = Html::parse_document(&body);

    if let Ok(content_selector) = Selector::parse("article")
        && let Some(element) = document.select(&content_selector).next()
    {
        let text = element.text().collect::<Vec<_>>().join(" ");
        return Ok(text);
    };

    if let Ok(fallback_selector) = Selector::parse("div.post-content")
        && let Some(el2) = document.select(&fallback_selector).next()
    {
        let text = el2.text().collect::<Vec<_>>().join(" ");
        return Ok(text);
    }

    let Ok(div_selector) = Selector::parse("div") else {
        return Err(anyhow!("Article extraction failed"));
    };

    let mut best_div = None;
    let mut max_len = 0;

    for div in document.select(&div_selector) {
        let text = div.text().collect::<String>();
        let len = text.trim().len();

        if len > max_len {
            max_len = len;
            best_div = Some(text);
        }
    }
    match best_div {
        Some(content) => Ok(replace_tags(&content).unwrap_or(content)),
        None => Err(anyhow!("No <div> found.")),
    }
}

fn replace_tags(content: &str) -> Result<String> {
    let re_tags = Regex::new(r"</?[^>]+>")?;
    let without_tags = re_tags.replace_all(content.as_bytes(), b"");
    let cleaned = String::from_utf8(without_tags.to_vec())?;

    let without_pipes = cleaned.replace("|", "");

    Ok(without_pipes
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" "))
}
