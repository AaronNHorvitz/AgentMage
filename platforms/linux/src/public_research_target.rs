//! URL-library normalization and complete destination checks inside the opt-in worker.
//! This module is compiled only by that binary, never the ordinary platform library.

use std::collections::BTreeSet;
use std::net::SocketAddr;

use agentmage_kernel_engine::research_budget::public_address;
use agentmage_kernel_engine::research_fetch::PublicGetTarget;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetError {
    Invalid,
    Redirect,
    Destination,
}

/// Structured fields are unencoded. Encode them exactly once with the URL library.
pub(crate) fn request_url(target: &PublicGetTarget) -> Result<Url, TargetError> {
    target.validate().map_err(|_| TargetError::Invalid)?;
    let mut url = Url::parse(&format!("https://{}{}", target.domain, target.path))
        .map_err(|_| TargetError::Invalid)?;
    if url.host_str() != Some(target.domain.as_str()) || url.path() != target.path {
        return Err(TargetError::Invalid);
    }
    if !target.query.is_empty() {
        url.query_pairs_mut().extend_pairs(&target.query);
    }
    validate_origin(&url, &target.domain)?;
    Ok(url)
}

fn validate_origin(url: &Url, domain: &str) -> Result<(), TargetError> {
    if url.scheme() != "https"
        || url.host_str() != Some(domain)
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(TargetError::Redirect);
    }
    Ok(())
}

/// Resolve a same-origin redirect with the established parser, then require a
/// lossless round-trip through the conservative request grammar. Never follow an
/// alternate host merely because it appears in the task's wider declared set.
pub(crate) fn redirect_target(
    current: &Url,
    location: &str,
    original_domain: &str,
) -> Result<PublicGetTarget, TargetError> {
    if location.is_empty()
        || location.len() > 4096
        || location
            .split('?')
            .next()
            .is_some_and(|path| path.contains('%'))
        || location
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || b == b'\\')
    {
        return Err(TargetError::Redirect);
    }
    let url = current.join(location).map_err(|_| TargetError::Redirect)?;
    validate_origin(&url, original_domain)?;
    let target = PublicGetTarget {
        domain: original_domain.to_owned(),
        path: url.path().to_owned(),
        query: url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect(),
    };
    // query_pairs uses UTF-8 replacement on invalid octets. Refuse lossy input
    // (and conservatively a literal replacement character) instead of changing
    // the server-supplied query into a different disclosure.
    if target
        .query
        .iter()
        .any(|(key, value)| key.contains('\u{fffd}') || value.contains('\u{fffd}'))
    {
        return Err(TargetError::Redirect);
    }
    let encoded = request_url(&target)?;
    // Query percent encoding can have equivalent spellings. Require the same
    // decoded fields and canonical path; the outgoing URL always uses our encoder.
    if encoded.path() != url.path()
        || encoded.query_pairs().collect::<Vec<_>>() != url.query_pairs().collect::<Vec<_>>()
    {
        return Err(TargetError::Redirect);
    }
    Ok(target)
}

/// Reject the whole result when any DNS answer is unsafe or the complete bounded
/// set cannot be established. Callers must pass all answers, not a library prefix.
pub(crate) fn public_destinations(
    answers: impl IntoIterator<Item = SocketAddr>,
) -> Result<Vec<SocketAddr>, TargetError> {
    let mut checked = BTreeSet::new();
    for (index, address) in answers.into_iter().enumerate() {
        let scoped = matches!(address, SocketAddr::V6(value) if value.scope_id() != 0 || value.flowinfo() != 0);
        if index >= 16 || address.port() != 443 || !public_address(address.ip()) || scoped {
            return Err(TargetError::Destination);
        }
        checked.insert(address);
    }
    if checked.is_empty() {
        return Err(TargetError::Destination);
    }
    Ok(checked.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> PublicGetTarget {
        PublicGetTarget {
            domain: "docs.example.com".into(),
            path: "/api".into(),
            query: vec![("q".into(), "version 2 & format=JSON".into())],
        }
    }

    #[test]
    fn query_disclosure_is_encoded_once_without_becoming_url_syntax() {
        let url = request_url(&target()).unwrap();
        assert_eq!(
            url.as_str(),
            "https://docs.example.com/api?q=version+2+%26+format%3DJSON"
        );
        assert_eq!(url.query_pairs().count(), 1);
        assert_eq!(url.query_pairs().next().unwrap().1, target().query[0].1);
    }

    #[test]
    fn structured_target_ambiguity_and_secrets_refuse_before_resolution() {
        for path in [
            "//other.test/",
            "/%2f%2fother.test",
            "/../admin",
            "/x#fragment",
            "/x?query",
        ] {
            let mut changed = target();
            changed.path = path.into();
            assert!(request_url(&changed).is_err());
        }
        let mut secret = target();
        secret.query[0].1 = format!("Bearer {}", "x".repeat(32));
        assert!(request_url(&secret).is_err());
    }

    #[test]
    fn redirect_preserves_origin_and_rejects_ambiguous_or_sensitive_disclosure() {
        let current = request_url(&target()).unwrap();
        let next = redirect_target(&current, "/v2?q=public+query", "docs.example.com").unwrap();
        assert_eq!(next.path, "/v2");
        assert_eq!(next.query, vec![("q".into(), "public query".into())]);
        for location in [
            "http://docs.example.com/",
            "https://elsewhere.example.com/",
            "//127.0.0.1/",
            "https://user@docs.example.com/",
            "https://docs.example.com:444/",
            "/x#fragment",
            "/x\\y",
            "/x\r\nHost: injected",
            "/x?q=1&q=2",
            "/%2e%2e/private",
            "/x?q=%FF",
        ] {
            assert!(
                redirect_target(&current, location, "docs.example.com").is_err(),
                "{location}"
            );
        }
    }

    #[test]
    fn complete_dns_set_must_be_public_on_the_exact_port() {
        let public: SocketAddr = "93.184.216.34:443".parse().unwrap();
        assert_eq!(public_destinations([public]).unwrap(), [public]);
        assert!(public_destinations([]).is_err());
        for unsafe_address in [
            "127.0.0.1:443",
            "10.1.2.3:443",
            "169.254.169.254:443",
            "[::1]:443",
            "[fc00::1]:443",
            "[fe80::1]:443",
            "[::ffff:127.0.0.1]:443",
            "93.184.216.34:80",
            "[2606:4700:4700::1111%1]:443",
        ] {
            let unsafe_address = unsafe_address.parse().unwrap();
            assert!(public_destinations([public, unsafe_address]).is_err());
            assert!(public_destinations([unsafe_address, public]).is_err());
        }
        // Even duplicate overflow is refused, never silently truncated at sixteen.
        assert!(public_destinations(std::iter::repeat_n(public, 17)).is_err());
    }
}
