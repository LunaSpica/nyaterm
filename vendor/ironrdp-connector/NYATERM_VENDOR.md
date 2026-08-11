# NyaTerm IronRDP Connector Vendor Note

- Upstream: `ironrdp-connector` 0.10.0 from the IronRDP 0.17 release family.
- Source: crates.io package `ironrdp-connector-0.10.0`.
- Local reason: the connector uses only `picky::key::PrivateKey`, but its unrestricted `picky` dependency enables default JOSE and PKCS12 features. Those unused defaults pin `aes-gcm 0.11.0-rc.4`, which cannot coexist in this workspace with NyaTerm's stable `aes-gcm 0.11.0` persistence dependency.
- Patch: use the API-compatible `picky 7.0.0-rc.26`, disable its default features, enable only `x509`, disable SSPI's unused `scard` feature, and make the connector's otherwise unreachable smart-card credential branch return `Unsupported`. Phase one explicitly excludes smart-card authentication/redirection; username/password NLA is unchanged.
- Validation: build `nyaterm-rdp-helper`, run workspace checks, and exercise NLA against the manual Windows matrix before release.
