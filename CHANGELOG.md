# Changelog

## [0.8.0] — 2026-09-04



## [0.7.4] — 2026-09-04

### Fixed

- fix(stats): the t-test p-value comes from Student's t distribution



## [0.7.3] — 2026-09-04

### Other

- docs(vo2max): the estimators have a capture path now, not a limitation



## [0.7.2] — 2026-09-04



## [Unreleased]

### Removed

- refactor(performance)!: `performance_analyzer_v2` is deleted — `PerformanceAnalyzerV2`
  and its result types (`FitnessScore`, `ActivityGoal`, `PerformancePrediction`,
  `TrainingLoadAnalysis`, `WeeklyLoad`). It was a second 0-100 fitness scorer beside
  `performance_analyzer::AdvancedPerformanceAnalyzer`, disagreeing with it on every
  constant (weights 0.5/0.3/0.2 vs 0.4/0.3/0.3, thresholds 75/60 vs 70/40, target
  weekly activities 4 vs 5, strength-endurance divisor 10 vs 5) and constructed
  nowhere.
- refactor(visitor)!: `ZoneTimeCalculator`, `ZoneBoundaries` and
  `ZoneDistributionResult` are deleted. They banded heart rate as a fraction of max
  HR, a zone model no caller used and which competed with the zone set the platform
  actually serves.
- refactor(config)!: `AlgorithmConfig::ftp`, `::lthr` and `::vo2max` are deleted with
  their serde defaults and the `PIERRE_FTP_ALGORITHM` / `PIERRE_LTHR_ALGORITHM` /
  `PIERRE_VO2MAX_ALGORITHM` overrides. No resolver read them, so setting one changed
  nothing; the algorithms they named take measured test inputs and cannot be chosen
  by name.
- `physiological_constants::hr_estimation::ASSUMED_MAX_HR` is deleted.

### Changed

- fix(vdot)!: VDOT uses Daniels & Gilbert's continuous %VO2max relation
  (`0.8 + 0.1894393 e^(-0.012778 t) + 0.2989558 e^(-0.1932605 t)`) in place of a
  five-bucket step function, and `predict_time` now inverts that same relation by
  bisection instead of applying a second table of velocity percentages. A 20:00 5K
  reads 49.8 rather than 47.5, and predictions land within 2.4% of Daniels'
  published times for VDOT 50 and 60.
- fix(recommendations)!: training-intensity classification uses the athlete's own
  maximum heart rate, derived from their profile age by the configured
  age-prediction formula, instead of a fixed 180 bpm. `TrainingPatternAnalysis`
  carries `intensity_balance: Option<f64>`, and a profile without an age gets no
  intensity verdict rather than one measured against an invented ceiling.

## [0.7.1] — 2026-09-04



## [0.7.0] — 2026-09-03

### Fixed

- fix(patterns)!: count weekdays and hours on the athlete's clock



## [0.6.0] — 2026-08-26

### Removed

- refactor(social)!: the social modules are deleted — `models::social`
  (`FriendConnection`, `SharedInsight`, `InsightReaction`, `AdaptedInsight`,
  `UserSocialSettings`, `InsightSharingPolicy`, `NotificationPreferences`,
  `FriendStatus`, `ShareVisibility`, `InsightType`, `ReactionType`,
  `TrainingPhase`, the request structs, `FriendInfo`, `FeedItem`,
  `ReactionSummary`), `insight_adapter` (`InsightAdapter`,
  `UserTrainingContext`, `AdaptationResult`, `truncate_string`) and
  `friend_activity_cache` (`FriendActivityCache`, `FriendActivitySummary`,
  `CacheConfig`, `CacheStats`, `DurationCategory`, `EffortLevel`,
  `create_shared_cache`). dravr-platform, the only consumer, retired the
  Insights and Friends surfaces by deletion, so nothing reads them.

## [0.5.5] — 2026-08-18

### Changed

- refactor(training-load)!: `TrainingStatus` is replaced by `FormBand`, one
  descriptive form vocabulary for every surface that bands form. The four old
  statuses become seven bands — `InsufficientHistory`, `DeepFatigue` (below
  -30% of CTL), `HeavyBlock` (-30%..-20%), `Productive` (-20%..-10%),
  `Balanced` (-10%..+5%), `Fresh` (+5%..+20%), `Detraining` (above +20%) —
  carrying the band edges consumers previously hand-rolled against raw TSB.
  `FormBand::form_pct`, `FormBand::from_tsb`, `FormBand::from_form_pct` and
  `FormBand::label` are the entry points; `TrainingLoadCalculator::interpret_tsb`
  is removed in favour of `FormBand::from_tsb`.

### Fixed

- fix(training-load): form is no longer banded on absolute TSB when there is
  no chronic base. `form_percentage` returned the raw TSB when CTL <= 1 and the
  bands were then applied to it, so a beginner's first hard week (CTL 0.5, TSB
  -35) read as an elite's deepest fatigue. `FormBand::form_pct` returns `None`
  there and the state is `InsufficientHistory`: `recommend_recovery_days`
  prescribes nothing and `check_overtraining_risk` raises no form factor,
  rather than deriving either from a number that cannot be interpreted.

## [0.5.4] — 2026-08-18

### Fixed

- fix(training-load): re-band TSB interpretation as form % of CTL —
  `interpret_tsb` and `recommend_recovery_days` now take `ctl` and band on
  `form_pct = tsb / ctl * 100` (absolute-TSB fallback when CTL <= 1), per the
  TrainingPeaks/Friel and intervals.icu convention: below -30% overreaching,
  -30%..+5% productive, +5%..+20% fresh, above +20% detraining; recovery days
  start past -30% (1/2/3 at -30/-40/-50%); `check_overtraining_risk` replaces
  the absolute ATL > 150 factor with ATL > 1.5x CTL and words factors as
  magnitude statements rather than injury claims (Impellizzeri 2020).
- fix: sidestep Rust 1.97 for_kv_map and bool_assert_comparison lints
- fix(deps): bump quinn-proto past the memory-exhaustion advisory
- fix: repair the SessionStart bootstrap guard for an empty .build
- fix(insight): char-safe truncate_string to avoid UTF-8 boundary panic

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
