use url::Url;

pub(crate) fn format_url(separator: &str, uri: &str) -> Option<String> {
    let url_parsed = Url::parse(uri).ok()?;
    let domain = url_parsed.host_str()?;

    let mut formatted_url = String::from(domain);
    if let Some(port) = url_parsed.port() {
        formatted_url.push_str(&format!("_{port}"));
    }
    formatted_url.push_str(&url_parsed.path().replace('/', separator));

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
    fn non_default_port_is_included_in_cache_name() {
        // El puerto no-default forma parte del nombre para evitar colisiones
        // entre mirrors del mismo host en distinto puerto.
        assert_eq!(
            format_url("_", "http://127.0.0.1:8080/x").as_deref(),
            Some("127.0.0.1_8080_x")
        );
        assert_eq!(
            format_url("_", "http://127.0.0.1/x").as_deref(),
            Some("127.0.0.1_x")
        );
    }

    #[test]
    fn default_port_is_omitted_from_cache_name() {
        // url::Url::port() devuelve None para el puerto por defecto del
        // esquema, así que los nombres existentes no cambian.
        assert_eq!(
            format_url("_", "http://example.com:80/a").as_deref(),
            Some("example.com_a")
        );
        assert_eq!(
            format_url("_", "https://example.com:443/a").as_deref(),
            Some("example.com_a")
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
