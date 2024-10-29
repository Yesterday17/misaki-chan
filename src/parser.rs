use once_cell::sync::Lazy;
use regex::Regex;

const OG_TITLE_REGEXP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"<meta\s+property="og:title"\s+content="([^"]+)"\s*/?>"#).unwrap());

pub async fn get_og_title(link: &str) -> anyhow::Result<Option<String>> {
    let html = reqwest::get(link).await?;
    let text = html.text().await?;
    let title = OG_TITLE_REGEXP
        .captures(&text)
        .and_then(|cap| cap.get(1))
        .map(|capture| {
            let capture = capture.as_str();
            // url decode
            html_escape::decode_html_entities(capture).to_string()
        });
    Ok(title)
}
