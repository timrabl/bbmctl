# bbmctl

Async Rust client library and CLI for the German [Breitbandmessung](https://breitbandmessung.de) broadband measurement API.

[![Test](https://github.com/timrabl/bbmctl/actions/workflows/test.yml/badge.svg)](https://github.com/timrabl/bbmctl/actions/workflows/test.yml)
[![Crates.io](https://img.shields.io/crates/v/bbm.svg)](https://crates.io/crates/bbm)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

## Background

This project started during a road trip through Norway and Sweden in 2023. Even at the North Cape, I had better mobile internet than in my hometown in Germany. That frustration led me to build a tool for measuring and tracking broadband speeds against what ISPs actually promise in their contracts.

The original version was written in Go. I shelved it between 2024 and 2025, then rewrote the entire project in Rust with a focus on performance and clean architecture -- async everywhere, concurrent speed measurements, and a proper library/CLI split.

This project is **not affiliated** with the German government or the Bundesnetzagentur. For more information about the official Breitbandmessung project, visit [breitbandmessung.de](https://breitbandmessung.de).

## Features

**Library (`bbm` crate):**
- Async HTTP client for the Breitbandmessung API
- Tower retry middleware with exponential backoff (configurable)
- Concurrent speed test runner using `FuturesUnordered` (configurable streams, default 8)
- TCP latency measurement with jitter calculation
- Plan comparison against contractual speed thresholds

**CLI (`bbmctl`):**
- Query providers, plans, and speeds from the API
- Run broadband speed tests with concurrent streams
- Compare measured speeds against your ISP contract
- Record measurements to a local SQLite database (SeaORM)
- Manage BNetzA measurement campaigns (Nachweisverfahren)
- Scheduled recurring tests (`test --every 30m`)
- Speed trend sparkline charts (`history trend`)
- Import/export measurement history as CSV
- Multiple output formats: table, JSON, YAML, CSV
- Config file with named profiles
- Shell completions for bash, zsh, fish, powershell, elvish
- Prometheus metrics exporter
- Human-readable speed display with auto unit detection

## Library usage

Add to your `Cargo.toml`:

```toml
[dependencies]
bbm = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use bbm::BbmClient;

#[tokio::main]
async fn main() -> Result<(), bbm::BbmError> {
    let client = BbmClient::new();

    // List all providers
    let providers = client.get_providers().await?;
    for provider in &providers {
        println!("{}: {}", provider.key, provider.value);
    }

    // Fetch plans for a specific provider
    let plans = client.get_plans_by_provider_id(437).await?;
    for plan in &plans {
        println!("Plan {}: max {}kbit/s down", plan.plan_id, plan.maxdownload);
    }

    Ok(())
}
```

## CLI installation

### From GitHub releases

Download the latest binary from [Releases](https://github.com/timrabl/bbmctl/releases).

### Homebrew (macOS)

```bash
brew install timrabl/tap/bbmctl
```

### From source

```bash
cargo install --git https://github.com/timrabl/bbmctl bbmctl
```

## CLI usage

```bash
# List providers
bbmctl list providers --search telekom

# Run a speed test
bbmctl test --duration 10 --streams 8

# Compare against your contract
bbmctl compare --provider 437 --plan 8515 --test

# Record measurements over time
bbmctl test --every 30m --provider 437

# View speed trends
bbmctl history trend --last 7d

# Start a BNetzA measurement campaign
bbmctl campaign start --provider 437 --plan 8515
bbmctl campaign test
bbmctl campaign report

# Export/import data
bbmctl history export -o backup.csv
bbmctl history import backup.csv

# Generate shell completions
bbmctl completions zsh > ~/.zfunc/_bbmctl
```

## Configuration

bbmctl looks for a config file at `~/.config/bbmctl/config.yaml` or `~/.bbmctl/config.yaml`:

```yaml
database: "~/.bbmctl/measurements.db"

default:
  provider: 437
  plan: "8515"
  format: table
  streams: 8
  duration: 10
  speed_unit: auto

profiles:
  office:
    provider: 251
    plan: "9001"
```

Use profiles with `--profile office`.

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | Error   |
| 2    | Threshold failure (compare/campaign test) |

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE).

## Links

- [API documentation](https://docs.rs/bbm)
- [Crates.io](https://crates.io/crates/bbm)
- [Issue tracker](https://github.com/timrabl/bbmctl/issues)
- [Breitbandmessung (official)](https://breitbandmessung.de)
