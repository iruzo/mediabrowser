use std::env::VarError;
use std::error::Error;
use std::net::IpAddr;

pub(super) enum Firewall {
    Whitelist(Vec<IpRange>),
    Blacklist(Vec<IpRange>),
}

pub(super) struct IpRange {
    start: IpAddr,
    end: IpAddr,
}

impl IpRange {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        let (address, prefix) = match value.split_once('/') {
            Some((address, prefix)) => (address, Some(prefix)),
            None => (value, None),
        };
        let address: IpAddr = address.parse()?;
        let bits = if address.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            Some(prefix) => {
                if !prefix.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("CIDR prefix must be a nonnegative integer".into());
                }
                prefix.parse::<u8>()?
            }
            None => bits,
        };
        if prefix > bits {
            return Err("CIDR prefix exceeds the address width".into());
        }
        let (start, end) = match address {
            IpAddr::V4(address) => {
                let mask = u32::MAX.checked_shr(u32::from(prefix)).unwrap_or(0);
                let start = u32::from(address) & !mask;
                (IpAddr::V4(start.into()), IpAddr::V4((start | mask).into()))
            }
            IpAddr::V6(address) => {
                let mask = u128::MAX.checked_shr(u32::from(prefix)).unwrap_or(0);
                let start = u128::from(address) & !mask;
                (IpAddr::V6(start.into()), IpAddr::V6((start | mask).into()))
            }
        };
        Ok(Self { start, end })
    }

    fn joins(&self, next: &Self) -> bool {
        match (self.end, next.start) {
            (IpAddr::V4(end), IpAddr::V4(start)) => {
                u32::from(start) <= u32::from(end).saturating_add(1)
            }
            (IpAddr::V6(end), IpAddr::V6(start)) => {
                u128::from(start) <= u128::from(end).saturating_add(1)
            }
            _ => false,
        }
    }
}

impl Firewall {
    pub(super) fn from_env() -> Result<Self, Box<dyn Error>> {
        match std::env::var("WHITELIST") {
            Ok(value) => return Ok(Self::Whitelist(parse_list("WHITELIST", &value)?)),
            Err(VarError::NotPresent) => {}
            Err(error) => return Err(error.into()),
        }
        match std::env::var("BLACKLIST") {
            Ok(value) => Ok(Self::Blacklist(parse_list("BLACKLIST", &value)?)),
            Err(VarError::NotPresent) => Ok(Self::Blacklist(Vec::new())),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn allows(&self, ip: IpAddr) -> bool {
        match self {
            Self::Whitelist(ranges) => contains(ranges, ip),
            Self::Blacklist(ranges) => !contains(ranges, ip),
        }
    }
}

fn parse_list(name: &str, value: &str) -> Result<Vec<IpRange>, Box<dyn Error>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut ranges = value
        .split(',')
        .map(|entry| {
            IpRange::parse(entry.trim()).map_err(|error| {
                format!("{name} contains an invalid IP or CIDR range {entry:?}: {error}").into()
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    ranges.sort_unstable_by_key(|range| range.start);
    // dedup_by removes the first argument; extend the retained previous range.
    ranges.dedup_by(|next, previous| {
        if previous.joins(next) {
            previous.end = previous.end.max(next.end);
            true
        } else {
            false
        }
    });
    Ok(ranges)
}

fn contains(ranges: &[IpRange], ip: IpAddr) -> bool {
    let index = ranges.partition_point(|range| range.start <= ip);
    index > 0 && ip <= ranges[index - 1].end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_rules() {
        const EXPECTED: &str = "MEDIABROWSER_FIREWALL_TEST_EXPECTED";

        if let Ok(expected) = std::env::var(EXPECTED) {
            let result = Firewall::from_env();
            if expected == "error" {
                assert!(result.is_err());
            } else {
                let firewall = result.unwrap();
                let allowed =
                    ["127.0.0.1", "127.0.0.2"].map(|ip| firewall.allows(ip.parse().unwrap()));
                assert_eq!(format!("{},{}", allowed[0], allowed[1]), expected);
            }
            return;
        }

        // Child processes keep environment changes isolated from parallel tests.
        let test = format!(
            "{}::environment_rules",
            module_path!().split_once("::").unwrap().1
        );
        for (whitelist, blacklist, expected) in [
            (None, None, "true,true"),
            (None, Some(""), "true,true"),
            (None, Some("  "), "true,true"),
            (None, Some("127.0.0.1"), "false,true"),
            (None, Some("invalid"), "error"),
            (Some("127.0.0.1"), None, "true,false"),
            (Some("127.0.0.1"), Some("127.0.0.1,127.0.0.2"), "true,false"),
            (Some("127.0.0.1"), Some("invalid"), "true,false"),
            (Some(""), None, "false,false"),
            (Some(""), Some("127.0.0.1"), "false,false"),
            (Some("  "), Some("invalid"), "false,false"),
            (Some("invalid"), Some(""), "error"),
        ] {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", &test, "--nocapture"])
                .env_remove("WHITELIST")
                .env_remove("BLACKLIST")
                .env(EXPECTED, expected);
            if let Some(value) = whitelist {
                command.env("WHITELIST", value);
            }
            if let Some(value) = blacklist {
                command.env("BLACKLIST", value);
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "WHITELIST={whitelist:?}, BLACKLIST={blacklist:?}:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn parses_exact_ips_with_whitespace() {
        let ips = parse_list("WHITELIST", " 127.0.0.1, 192.168.1.20, ::1 ").unwrap();
        assert_eq!(ips.len(), 3);
        assert_eq!(ips[0].start, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(ips[1].start, "192.168.1.20".parse::<IpAddr>().unwrap());
        assert_eq!(ips[2].start, "::1".parse::<IpAddr>().unwrap());
        assert!(parse_list("WHITELIST", "").unwrap().is_empty());
        assert!(parse_list("BLACKLIST", "  ").unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_entries() {
        for value in [
            "localhost",
            "127.0.0.1:8080",
            "192.168.1.0/33",
            "::/129",
            "127.0.0.1/",
            "127.0.0.1/-1",
            "127.0.0.1/+24",
            "127.0.0.1/24/32",
            "127.0.0.1/abc",
            "127.0.0.1/256",
            "127.0.0.1-127.0.0.10",
            "*",
            "256.0.0.1",
            ",127.0.0.1",
            "127.0.0.1,",
            "127.0.0.1,,192.168.1.20",
        ] {
            assert!(
                parse_list("WHITELIST", value).is_err(),
                "accepted {value:?}"
            );
        }
    }

    #[test]
    fn matches_cidr_boundaries_and_address_families() {
        for (range, ip, expected) in [
            ("192.168.1.0/24", "192.168.1.0", true),
            ("192.168.1.0/24", "192.168.1.255", true),
            ("192.168.1.0/24", "192.168.0.255", false),
            ("192.168.1.0/24", "192.168.2.0", false),
            ("192.168.1.42/24", "192.168.1.200", true),
            ("192.168.1.0/25", "192.168.1.127", true),
            ("192.168.1.0/25", "192.168.1.128", false),
            ("192.168.1.0/31", "192.168.1.1", true),
            ("192.168.1.0/31", "192.168.1.2", false),
            ("192.168.1.1/32", "192.168.1.1", true),
            ("192.168.1.1/32", "192.168.1.2", false),
            ("0.0.0.0/0", "255.255.255.255", true),
            ("0.0.0.0/0", "::1", false),
            (
                "2001:db8::/32",
                "2001:db8:ffff:ffff:ffff:ffff:ffff:ffff",
                true,
            ),
            ("2001:db8::/32", "2001:db9::", false),
            ("2001:db8::1/127", "2001:db8::", true),
            ("2001:db8::1/127", "2001:db8::2", false),
            ("::1/128", "::1", true),
            ("::1/128", "::2", false),
            ("::/0", "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff", true),
            ("::/0", "127.0.0.1", false),
        ] {
            assert_eq!(
                contains(
                    &parse_list("WHITELIST", range).unwrap(),
                    ip.parse().unwrap()
                ),
                expected,
                "{range}: {ip}"
            );
        }
    }

    #[test]
    fn merges_unsorted_overlapping_and_adjacent_ranges() {
        let value = "192.0.2.8,192.0.2.4/30,192.0.2.1/30,192.0.2.0/31,192.0.2.8,\
                     192.0.2.10,2001:db8::8,2001:db8::4/126,2001:db8::1/126,\
                     2001:db8::/127,2001:db8::8,2001:db8::a";
        let ranges = parse_list("WHITELIST", value).unwrap();
        assert_eq!(ranges.len(), 4);
        for (range, (start, end)) in ranges.iter().zip([
            ("192.0.2.0", "192.0.2.8"),
            ("192.0.2.10", "192.0.2.10"),
            ("2001:db8::", "2001:db8::8"),
            ("2001:db8::a", "2001:db8::a"),
        ]) {
            assert_eq!(range.start, start.parse::<IpAddr>().unwrap());
            assert_eq!(range.end, end.parse::<IpAddr>().unwrap());
        }
        for ip in [
            "192.0.1.255",
            "192.0.2.9",
            "192.0.2.11",
            "2001:db8::9",
            "2001:db8::b",
        ] {
            assert!(!contains(&ranges, ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "192.0.2.0",
            "192.0.2.5",
            "192.0.2.8",
            "192.0.2.10",
            "2001:db8::",
            "2001:db8::5",
            "2001:db8::8",
            "2001:db8::a",
        ] {
            assert!(contains(&ranges, ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn handles_full_ranges_and_maximum_addresses() {
        for value in [
            "255.255.255.255,255.255.255.254/31,0.0.0.0/0,0.0.0.0/0",
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff,::/0,::/0",
        ] {
            assert_eq!(parse_list("WHITELIST", value).unwrap().len(), 1);
        }
        let ranges = parse_list("WHITELIST", "255.255.255.255,::").unwrap();
        assert_eq!(ranges.len(), 2);
        assert!(!contains(&ranges, "255.255.255.254".parse().unwrap()));
        assert!(!contains(&ranges, "::1".parse().unwrap()));
        let ranges = parse_list("WHITELIST", "::/0,0.0.0.0/0").unwrap();
        assert_eq!(ranges.len(), 2);
        for ip in [
            "0.0.0.0",
            "255.255.255.255",
            "::",
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
        ] {
            assert!(contains(&ranges, ip.parse().unwrap()), "{ip}");
        }
        // A catch-all must not hide invalid entries later in the active list.
        assert!(parse_list("WHITELIST", "0.0.0.0/0,::/0,invalid").is_err());
    }

    #[test]
    fn lookup_matches_linear_cidr_scan() {
        use std::fmt::Write;

        let rules = [
            (240, 4),
            (17, 3),
            (9, 0),
            (32, 4),
            (0, 2),
            (16, 2),
            (48, 3),
            (9, 0),
        ];
        let mut value = String::new();
        for (address, host_bits) in rules {
            if !value.is_empty() {
                value.push(',');
            }
            write!(
                value,
                "192.0.2.{address}/{},2001:db8::{address:x}/{}",
                32 - host_bits,
                128 - host_bits
            )
            .unwrap();
        }
        let ranges = parse_list("WHITELIST", &value).unwrap();
        for address in 0..=255u32 {
            let expected = rules
                .iter()
                .any(|(network, host_bits)| address >> host_bits == network >> host_bits);
            for ip in [
                IpAddr::V4((0xc0000200u32 | address).into()),
                IpAddr::V6((0x20010db8000000000000000000000000u128 | u128::from(address)).into()),
            ] {
                assert_eq!(contains(&ranges, ip), expected, "{ip}");
            }
        }
    }

    #[test]
    fn mixes_ranges_with_exact_ips() {
        let value = " 192.168.1.0/24, 10.0.0.5, 2001:db8::/32 ";
        let whitelist = Firewall::Whitelist(parse_list("WHITELIST", value).unwrap());
        let blacklist = Firewall::Blacklist(parse_list("BLACKLIST", value).unwrap());
        for (ip, listed) in [
            ("192.168.1.100", true),
            ("10.0.0.5", true),
            ("2001:db8::1", true),
            ("192.168.2.100", false),
            ("10.0.0.6", false),
            ("2001:db9::1", false),
        ] {
            let ip = ip.parse().unwrap();
            assert_eq!(whitelist.allows(ip), listed);
            assert_eq!(blacklist.allows(ip), !listed);
        }
    }

    #[test]
    fn whitelist_allows_only_listed_ips() {
        let firewall =
            Firewall::Whitelist(parse_list("WHITELIST", "127.0.0.1,192.168.1.20").unwrap());
        assert!(firewall.allows("127.0.0.1".parse().unwrap()));
        assert!(firewall.allows("192.168.1.20".parse().unwrap()));
        assert!(!firewall.allows("127.0.0.2".parse().unwrap()));
        assert!(!Firewall::Whitelist(Vec::new()).allows("127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn blacklist_denies_only_listed_ips() {
        let firewall =
            Firewall::Blacklist(parse_list("BLACKLIST", "127.0.0.1,192.168.1.20").unwrap());
        assert!(!firewall.allows("127.0.0.1".parse().unwrap()));
        assert!(!firewall.allows("192.168.1.20".parse().unwrap()));
        assert!(firewall.allows("127.0.0.2".parse().unwrap()));
        assert!(Firewall::Blacklist(Vec::new()).allows("127.0.0.1".parse().unwrap()));
    }
}
