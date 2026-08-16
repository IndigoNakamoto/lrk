use axum::http::Uri;

pub fn api_bases(input: &str) -> Result<Vec<String>, String> {
    let input = input.trim().trim_end_matches('/');
    if input.is_empty() {
        return Err("REST API URL or host must not be empty".to_string());
    }
    let bases = if input.contains("://") {
        vec![input.to_string()]
    } else {
        vec![format!("https://{input}"), format!("http://{input}")]
    };
    for base in &bases {
        validate_api_base(base)?;
    }
    Ok(bases)
}

fn validate_api_base(base: &str) -> Result<(), String> {
    let uri: Uri = base
        .parse()
        .map_err(|error| format!("invalid REST API origin: {error}"))?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err("REST API origin must be an absolute HTTP(S) URL".to_string());
    }
    if uri.query().is_some() || !matches!(uri.path(), "" | "/") {
        return Err("REST API origin must not contain a path or query".to_string());
    }
    if uri
        .authority()
        .is_some_and(|authority| authority.as_str().rsplit_once('@').is_some())
    {
        return Err("REST API origin must not contain credentials".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_origin_only_base_urls() {
        assert!(validate_api_base("https://bitview.space").is_ok());
        assert!(validate_api_base("http://127.0.0.1:3110").is_ok());
        assert!(validate_api_base("https://bitview.space/api").is_err());
        assert!(validate_api_base("https://user:pass@bitview.space").is_err());
    }

    #[test]
    fn expands_bare_hosts_with_https_first() {
        assert_eq!(
            api_bases("bitview.space").unwrap(),
            ["https://bitview.space", "http://bitview.space"]
        );
        assert_eq!(
            api_bases("http://127.0.0.1:3110/").unwrap(),
            ["http://127.0.0.1:3110"]
        );
    }

    #[test]
    fn reports_positional_origin_errors() {
        assert_eq!(
            validate_api_base("bitview.space").unwrap_err(),
            "REST API origin must be an absolute HTTP(S) URL"
        );
        assert_eq!(
            validate_api_base("https://bitview.space/api").unwrap_err(),
            "REST API origin must not contain a path or query"
        );
        assert_eq!(
            validate_api_base("https://user:pass@bitview.space").unwrap_err(),
            "REST API origin must not contain credentials"
        );
    }
}
