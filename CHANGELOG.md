# Changelog

## [0.5.3] — 2026-06-19

### Changed

- deps: migrate `dravr-cageux-mcp` and `dravr-cageux-server` to dravr-tronc
  0.5.3 (dual-era MCP engine); state is `Arc<S>` directly (tronc no longer wraps
  it in a `RwLock`). The core `dravr-cageux` crate is unchanged.

## [0.5.2] — 2026-05-30



## [0.5.1] — 2026-05-30



## [0.5.0] — 2026-05-29

### Added

- feat(algorithms): make all 9 algorithms config/env-selectable with tunable params Adds selection fields + AlgorithmParamsConfig + resolvers, wires every consumption site, unifies the duplicate training-load EMA/TssDataPoint into TrainingLoadAlgorithm, and fixes pre-existing suboptimal_flops nursery lints.



## [0.4.0] — 2026-04-24

### Added

- feat(activity): add Split + Lap types and Activity.splits/laps fields Detailed-endpoint providers (Strava, Garmin) return per-km/mi splits and athlete-triggered laps; cageux now models both with accessors and builder methods so downstream crates can surface per-lap HR/pace/power and split elevation deltas without bespoke JSON handling.

### Fixed

- fix: sidestep Rust 1.95 unnecessary_sort_by + map_unwrap_or lints sort_by_key(|b| Reverse(b.x)) for reverse ordering, map_or for Result::map().unwrap_or().



## [0.3.0] — 2026-04-13



## [0.2.0] — 2026-04-11

### Added

- feat: add seasonality module for location-aware sport recommendations Season detection, 36-sport compatibility matrix, alternative suggestions, 24 tests



## [0.1.8] — 2026-04-10

### Other

- build: reduce tokio feature footprint to minimal set



## [0.1.7] — 2026-04-09



## [0.1.5] — 2026-04-09



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
