# Cageux — Sports Science Intelligence Engine

[![CI](https://github.com/dravr-ai/dravr-cageux/actions/workflows/ci.yml/badge.svg)](https://github.com/dravr-ai/dravr-cageux/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE.md)

Standalone Rust library for sports science analysis: VDOT, TSS, TRIMP, FTP, VO2max, recovery scoring, sleep analysis, nutrition planning, and performance prediction. Pure algorithms with zero database or HTTP dependencies in the core crate.

## Table of Contents

- [Quick Start](#quick-start)
- [Algorithms](#algorithms)
- [REST API Server](#rest-api-server-dravr-cageux-server)
- [MCP Server](#mcp-server-dravr-cageux-mcp)
- [Library Usage](#library-usage-rust)
- [Data Models](#data-models)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [License](#license)

## Quick Start

### Library (Rust)

```toml
[dependencies]
dravr-cageux = { git = "https://github.com/dravr-ai/dravr-cageux.git", tag = "v0.1.0" }
```

```rust
use dravr_cageux::models::{ActivityBuilder, SportType};
use dravr_cageux::algorithms::ftp::FtpAlgorithm;
use chrono::Utc;

// Build an activity
let activity = ActivityBuilder::new("123", "Morning Run", SportType::Run, Utc::now(), 1800, "strava")
    .distance_meters(5000.0)
    .average_heart_rate(150)
    .average_power(250)
    .build();

// Estimate FTP from a 20-minute power test
let ftp = FtpAlgorithm::From20MinTest { avg_power_20min: 300.0 }
    .estimate_ftp()
    .unwrap();
assert!((ftp - 285.0).abs() < 0.1); // 300 * 0.95
```

### REST API Server

```bash
cargo run --bin dravr-cageux-server -- serve --port 3100
```

```bash
curl http://localhost:3100/health
# {"status":"ok","service":"dravr-cageux","version":"0.1.0"}
```

### MCP Server (stdio)

```bash
cargo run --bin dravr-cageux-mcp -- --transport stdio
```

Or over HTTP:

```bash
cargo run --bin dravr-cageux-mcp -- --transport http --port 3100
```

## Algorithms

| Algorithm | Module | Description |
|-----------|--------|-------------|
| **MaxHR** | `models::maxhr` | Fox, Tanaka, Nes, Gulati formulas for age-predicted max heart rate |
| **FTP** | `algorithms::ftp` | Functional Threshold Power from 20min test, 8min test, ramp test, Critical Power model |
| **TSS** | `algorithms::tss` | Training Stress Score from average power, normalized power, or hybrid |
| **TRIMP** | `algorithms::trimp` | Training Impulse (Bannister male/female, Edwards simplified, Lucia banded) |
| **VDOT** | `algorithms::vdot` | Jack Daniels' VDOT running performance index |
| **VO2max** | `algorithms::vo2max` | VO2max estimation from VDOT, Cooper test, Rockport walk, Astrand-Ryhming |
| **LTHR** | `algorithms::lthr` | Lactate Threshold Heart Rate from max HR, 30min test, ramp test |
| **Recovery** | `algorithms::recovery_aggregation` | Multi-metric recovery score (HRV, sleep, HR recovery, fatigue) |
| **Training Load** | `algorithms::training_load` | ATL/CTL/TSB training load balance and overtraining risk |

### Analysis Modules

| Module | Description |
|--------|-------------|
| `analyzer` | Single-activity analysis with zone distribution and insights |
| `activity_analyzer` | Advanced contextual analysis with trend comparison |
| `performance_analyzer_v2` | Multi-activity performance trends and fitness scoring |
| `performance_prediction` | Race time prediction using regression models |
| `pattern_detection` | Hard/easy balance, overtraining signals, volume trends |
| `recovery_calculator` | Recovery score (0-100) with rest day recommendations |
| `sleep_analysis` | Sleep quality scoring and HRV trend detection |
| `nutrition_calculator` | TDEE, BMR (Mifflin-St Jeor), macronutrient planning |
| `training_load` | TSS-based training load with overtraining risk assessment |
| `statistical_analysis` | Linear regression, significance testing, trend detection |
| `visitor` | Single-pass time series processing (normalized power, zone time, stats) |

## Data Models

### Activity

The `Activity` struct represents a single fitness session from any provider. Use `ActivityBuilder` to construct:

```rust
use dravr_cageux::models::{ActivityBuilder, SportType};
use chrono::Utc;

let activity = ActivityBuilder::new("id", "Evening Ride", SportType::Ride, Utc::now(), 3600, "garmin")
    .distance_meters(40_000.0)
    .elevation_gain(800.0)
    .average_heart_rate(155)
    .average_power(250)
    .ftp(280)
    .city("Montreal".to_owned())
    .build();
```

Fields include: heart rate (avg/max), power (avg/max/normalized), cadence, speed, elevation, HRV, temperature, GPS, segment efforts, and time-series data.

### SportType

36 sport types: `Run`, `Ride`, `Swim`, `Walk`, `Hike`, `VirtualRide`, `VirtualRun`, `MountainBike`, `GravelRide`, `TrailRunning`, `CrossCountrySkiing`, `Yoga`, `StrengthTraining`, `Crossfit`, and more. `Other(String)` handles provider-specific types.

### TimeSeriesData

Second-by-second streams: heart rate, power, cadence, speed, altitude, temperature, GPS coordinates.

## Configuration

All server configuration is loaded from environment variables (`.envrc` + direnv):

| Variable | Default | Description |
|----------|---------|-------------|
| `CAGEUX_HOST` | `127.0.0.1` | Server bind address |
| `CAGEUX_PORT` | `3100` | Server listen port |
| `CAGEUX_TRANSPORT` | `http` | MCP transport mode (`stdio` or `http`) |
| `CAGEUX_API_TOKEN` | *(none)* | Bearer token for REST API auth (empty = no auth) |
| `RUST_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |

Example `.envrc`:

```bash
export CAGEUX_HOST="127.0.0.1"
export CAGEUX_PORT="3100"
export RUST_LOG="dravr_cageux=info,dravr_cageux_mcp=info,dravr_cageux_server=info"
export CAGEUX_TRANSPORT="http"
export CAGEUX_API_TOKEN=""
```

## Architecture

```
dravr-cageux/
├── src/                        # Core library (pure algorithms, zero I/O deps)
│   ├── lib.rs                  # Public API and module declarations
│   ├── error.rs                # IntelligenceError structured error types
│   ├── config/                 # Server config + intelligence analysis config
│   ├── constants/              # Physiological thresholds, time conversions
│   ├── models/                 # Activity, SportType, TimeSeriesData, fitness profiles
│   ├── algorithms/             # FTP, TSS, TRIMP, VDOT, VO2max, LTHR, MaxHR
│   ├── types.rs                # Shared analysis types (trends, goals, insights)
│   ├── analyzer.rs             # Single-activity analysis
│   ├── performance_analyzer_v2.rs  # Multi-activity performance trends
│   ├── recovery_calculator.rs  # Recovery scoring
│   ├── sleep_analysis.rs       # Sleep quality + HRV
│   ├── nutrition_calculator.rs # TDEE, BMR, macros
│   ├── visitor.rs              # Single-pass time series processing
│   └── ...                     # Pattern detection, goal engine, recommendations
│
├── crates/
│   ├── dravr-cageux-mcp/       # MCP server (JSON-RPC 2.0, stdio + HTTP)
│   │   ├── src/server.rs       # JSON-RPC router (initialize, tools/list, tools/call)
│   │   ├── src/tools/          # McpTool trait + ToolRegistry
│   │   └── src/transport/      # StdioTransport, HttpTransport
│   │
│   └── dravr-cageux-server/    # Unified REST API + MCP server
│       ├── src/router.rs       # Axum routes (/health, /mcp)
│       ├── src/auth.rs         # Bearer token middleware
│       └── src/main.rs         # CLI (serve, stdio)
│
└── tests/                      # Integration tests (algorithms, models, E2E server)
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
