# Firewall

The `firewall` feature filters connections by socket peer IP before HTTP or TLS
handling. It has no additional dependencies.

```sh
WHITELIST=127.0.0.1,192.168.1.20 cargo run --locked --features firewall
BLACKLIST=192.168.1.0/24,10.0.0.5 cargo run --locked --features firewall
```

Both variables accept comma-separated IP addresses or CIDR ranges with optional
whitespace. IPv4 and IPv6 entries are supported:

```sh
WHITELIST=127.0.0.1,::1,2001:db8::/32 cargo run --locked --features firewall
```

If `WHITELIST` is set, only matching clients are allowed and `BLACKLIST` is
ignored. An empty whitelist blocks everyone. Otherwise, `BLACKLIST` blocks
matching clients; an unset or empty blacklist allows everyone. Invalid entries
in the active list stop startup.

Lists are read once at startup, sorted, and merged. Connections use an
allocation-free binary search. IPv4 peers reported as IPv4-mapped IPv6 addresses
on dual-stack sockets are matched against IPv4 rules.

Blocked connections are closed before HTTP handling or the TLS handshake. The
filter uses the socket peer IP, not forwarded HTTP headers. Without this feature,
