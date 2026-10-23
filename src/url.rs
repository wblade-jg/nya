use url::Url;

pub(crate) fn format_url(separator: &str, uri: &str) -> Option<String> {
    let url_parsed = Url::parse(uri).ok()?;
    let domain = url_parsed.host_str()?;

    let formatted_url = format!("{}{}", domain, url_parsed.path().replace('/', separator));

    Some(formatted_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_normal_url_with_path() {
        assert_eq!(
            format_url("_", "http://example.com/ubuntu").as_deref(),
            Some("example.com_ubuntu")
        );
    }

    #[test]
    fn root_path_becomes_trailing_separator() {
        assert_eq!(
            format_url("_", "http://example.com").as_deref(),
            Some("example.com_")
        );
    }

    #[test]
    fn invalid_url_returns_none() {
        assert_eq!(format_url("_", "not a url"), None);
        assert_eq!(format_url("_", ""), None);
    }

    #[test]
    fn url_without_host_returns_none() {
        assert_eq!(format_url("_", "file:///etc/apt"), None);
    }

    #[test]
    fn duplicate_slashes_produce_double_separator() {
        assert_eq!(
            format_url("_", "http://example.com//a").as_deref(),
            Some("example.com__a")
        );
    }

    #[test]
    fn port_is_discarded_ignoring_cache_collision() {
        // Documenta la limitación actual: el puerto no forma parte del nombre,
        // así que mirrors en distinto puerto colisionan en la caché plana.
        assert_eq!(
            format_url("_", "http://127.0.0.1:8080/x").as_deref(),
            Some("127.0.0.1_x")
        );
        assert_eq!(
            format_url("_", "http://127.0.0.1/x").as_deref(),
            Some("127.0.0.1_x")
        );
    }

    #[test]
    fn paths_slash_vs_underscore_collide() {
        // Documenta la limitación actual: '/a/b' y '/a_b' producen el
        // mismo nombre de fichero en la caché.
        assert_eq!(
            format_url("_", "http://example.com/a/b"),
            format_url("_", "http://example.com/a_b")
        );
    }
}
