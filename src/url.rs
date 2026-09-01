use url::Url;

pub(crate) fn format_url(separator: &str, uri: &str) -> Option<String> {
    let url_parsed = Url::parse(uri).ok()?;
    let domain = url_parsed.host_str()?;

    let formatted_url = format!(
        "{}{}",
        domain,
        url_parsed.path().replace('/', separator)
    );

    Some(formatted_url)
}
