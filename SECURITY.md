# Security Policy

## Supported versions

This project is under active development. Until versioned releases are published, security fixes are applied to the default development branch when they are verified.

| Version | Security support |
| --- | --- |
| Default development branch | Supported |
| Unreleased feature branches | Best effort |
| Old snapshots or forks | Not supported |

## Reporting a vulnerability

Please report suspected vulnerabilities privately. Do not open a public GitHub issue, pull request, discussion, or commit containing exploit details.

Preferred channels, in order:

1. Open a **private GitHub Security Advisory** for the repository once the repository is available on GitHub.
2. If private advisories are unavailable, contact the repository maintainer through the private account/contact channel associated with the repository and include the subject `SECURITY: rust-kernel-game-engine-kit`.

If neither private channel is available, provide only a minimal public issue stating that a security report is pending and request a private contact route. Do not include exploit code, secrets, or reproduction details in that issue.

## What to include

Please include enough information to reproduce and assess the issue safely:

- Affected commit, version, target OS, architecture, Rust toolchain, and feature flags.
- A concise description of the impact and attack surface.
- Minimal reproduction steps or a reduced test case, preferably without weaponized payloads.
- Whether the issue involves memory safety, denial of service, data corruption, authentication, replay, secret handling, or native-handle lifetime.
- Any known mitigation or proposed fix.

Encrypt sensitive attachments when the reporting channel supports encryption. Never include real production keys, credentials, user data, or private save files.

## Response process

Maintainers will acknowledge a private report when possible, reproduce it, determine severity and affected versions, and coordinate a fix or mitigation. Timelines depend on reproducibility, impact, and the availability of the affected platform.

Security fixes should include a regression test where practical. Native Windows/Linux issues must identify the exact platform and driver/runtime because a headless test cannot prove native Vulkan, io_uring, IOCP, X11, or Win32 safety.

## Security boundaries in this project

The following areas receive special scrutiny:

- Fallible allocation, checked arithmetic, queue bounds, and allocator fragmentation.
- Vulkan, raw syscall, io_uring, IOCP, and OS-handle ownership/lifetimes.
- Sync packet parsing, HMAC verification, session nonce validation, replay behavior, and WAL recovery/compaction.
- Fuzz targets, deserialization boundaries, message-bus routing, and dependency topology.
- CI actions, dependency changes, generated artifacts, and secret exposure.

HMAC provides integrity/authentication only; it does not provide confidentiality. Do not report the absence of encryption as a bug unless the product integration claims confidentiality without an AEAD transport.
