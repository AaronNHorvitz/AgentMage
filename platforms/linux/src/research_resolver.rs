//! Minimal operation-scoped resolver projection for the native sandbox owner.
//! No lookup, permission, socket or host NSS plugin is introduced by preparation.

use std::collections::BTreeSet;
use std::fmt;
use std::net::IpAddr;

const MAX_RESOLVER_BYTES: usize = 16 * 1024;
const MAX_NAMESERVERS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResolverProjectionError;

/// Minimal configuration bytes, not a trusted input file, grant or launch capability.
pub(crate) struct ResolverProjection {
    resolv_conf: Vec<u8>,
    count: usize,
}

impl fmt::Debug for ResolverProjection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolverProjection")
            .field("nameserver_count", &self.count)
            .finish_non_exhaustive()
    }
}

impl ResolverProjection {
    /// The native manifest owner must independently pin the original resolver file,
    /// seal these derived bytes and project only the two exact configuration files.
    /// No ambient search suffix, host file, NSS plugin, proxy or private comment survives.
    pub(crate) fn prepare(source: &[u8]) -> Result<Self, ResolverProjectionError> {
        if source.is_empty() || source.len() > MAX_RESOLVER_BYTES {
            return Err(ResolverProjectionError);
        }
        let source = std::str::from_utf8(source).map_err(|_| ResolverProjectionError)?;
        if source
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err(ResolverProjectionError);
        }
        let mut nameservers = Vec::new();
        let mut seen = BTreeSet::new();
        for line in source.lines() {
            let line = line
                .split(['#', ';'])
                .next()
                .ok_or(ResolverProjectionError)?;
            let mut fields = line.split_ascii_whitespace();
            let Some(directive) = fields.next() else {
                continue;
            };
            match directive {
                "nameserver" => {
                    let value = fields.next().ok_or(ResolverProjectionError)?;
                    let address: IpAddr = value.parse().map_err(|_| ResolverProjectionError)?;
                    if fields.next().is_some()
                        || address.is_unspecified()
                        || address.is_multicast()
                        || matches!(address, IpAddr::V4(v4) if v4.is_broadcast())
                        || matches!(address, IpAddr::V6(v6) if v6.to_ipv4_mapped().is_some() || v6.is_unicast_link_local())
                        || !seen.insert(address)
                        || nameservers.len() == MAX_NAMESERVERS
                    {
                        return Err(ResolverProjectionError);
                    }
                    nameservers.push(address);
                }
                // These host directives are deliberately not copied or interpreted.
                // Output options below are fixed and cannot add lookup destinations.
                "domain" | "search" | "options" | "sortlist" => {}
                _ => return Err(ResolverProjectionError),
            }
        }
        if nameservers.is_empty() {
            // Never permit libc's implicit default resolver or an ambient fallback.
            return Err(ResolverProjectionError);
        }
        let mut output = String::from("# AgentMage operation-scoped resolver\n");
        for address in &nameservers {
            output.push_str(&format!("nameserver {address}\n"));
        }
        output.push_str("options timeout:1 attempts:1 ndots:0\n");
        Ok(Self {
            resolv_conf: output.into_bytes(),
            count: nameservers.len(),
        })
    }

    pub(crate) fn resolv_conf(&self) -> &[u8] {
        &self.resolv_conf
    }

    pub(crate) const fn nsswitch_conf(&self) -> &'static [u8] {
        b"hosts: dns\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_system_resolver_is_preserved_but_search_options_and_comments_are_not() {
        let source = b"# private synthetic comment\nsearch private.example.test\nnameserver 127.0.0.53\noptions rotate ndots:5 attempts:9\n";
        let projection = ResolverProjection::prepare(source).unwrap();
        assert_eq!(projection.resolv_conf(), b"# AgentMage operation-scoped resolver\nnameserver 127.0.0.53\noptions timeout:1 attempts:1 ndots:0\n");
        assert_eq!(projection.nsswitch_conf(), b"hosts: dns\n");
        assert!(!format!("{projection:?}").contains("127.0.0.53"));
        assert!(!format!("{projection:?}").contains("private"));
    }

    #[test]
    fn complete_bounded_ordered_server_set_is_not_truncated_or_sorted() {
        let source = b"nameserver 192.0.2.2\nnameserver 192.0.2.1\nnameserver 2001:db8::1\n";
        let projection = ResolverProjection::prepare(source).unwrap();
        assert_eq!(projection.count, 3);
        let output = std::str::from_utf8(projection.resolv_conf()).unwrap();
        assert!(output.find("192.0.2.2").unwrap() < output.find("192.0.2.1").unwrap());
        let too_many = [source.as_slice(), b"nameserver 192.0.2.3\n"].concat();
        assert!(ResolverProjection::prepare(&too_many).is_err());
    }

    #[test]
    fn missing_malformed_scoped_ambiguous_and_oversized_inputs_fail_closed() {
        for source in [
            "",
            "# no resolver",
            "search example.test\n",
            "nameserver\n",
            "nameserver dns.example.test\n",
            "nameserver 127.0.0.1:53\n",
            "nameserver [::1]\n",
            "nameserver fe80::1%eth0\n",
            "nameserver fe80::1\n",
            "nameserver ::ffff:127.0.0.1\n",
            "nameserver ::ffff:224.0.0.1\n",
            "nameserver 0.0.0.0\n",
            "nameserver ::\n",
            "nameserver 224.0.0.1\n",
            "nameserver 255.255.255.255\n",
            "nameserver ff02::1\n",
            "nameserver 127.0.0.53 extra\n",
            "nameserver 127.0.0.53\nnameserver 127.0.0.53\n",
            "nameserver 127.0.0.53\nlookup file bind\n",
            "nameserver 127.0.0.53\0\n",
        ] {
            assert!(
                ResolverProjection::prepare(source.as_bytes()).is_err(),
                "{source:?}"
            );
        }
        assert!(ResolverProjection::prepare(&vec![b' '; MAX_RESOLVER_BYTES + 1]).is_err());
        assert!(ResolverProjection::prepare(&[0xff]).is_err());
    }
}
