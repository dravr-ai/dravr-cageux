# Changelog

## [0.1.4] — 2026-04-09

### Fixed

- fix: use sport-type-aware pace baselines for TSS fallback estimation Single running baseline inflated cycling TSS ~5x; each sport now has its own moderate-effort baseline



## [0.1.3] — 2026-03-31

### Added

- feat: integrate dravr-build-config for shared validation and lint rules



## [0.1.2] — 2026-03-26

### Other

- deps: bump dravr-tronc to 0.2 with error notification support



## [0.1.1] — 2026-03-23

### Other

- refactor: adopt dravr-tronc shared MCP infrastructure



## [0.1.0] — 2026-03-22

### Added

- feat: extract all intelligence algorithms and analysis modules from pierre-intelligence 21K lines: VDOT, TSS, TRIMP, FTP, VO2max, recovery, sleep, nutrition, pattern detection, performance analysis, visitor pattern

### Fixed

- fix: change social enum FromStr error type from IntelligenceError to String



All notable changes to this project will be documented in this file.
Versions are managed by the [release workflow](.github/workflows/release.yml).
