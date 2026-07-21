# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0](https://github.com/timrabl/bbmctl/compare/v0.1.2...v0.2.0) - 2026-07-21

The published `bbm` library API is unchanged since 0.1.2; the entry below it is
what release-plz records for crates.io. The version bump to 0.2.0 reflects the
new CLI capability and the move to a stable database backend. As before, the
`bbmctl` binaries attached to this release carry the changes that release-plz
does not track (`bbmctl` and `bbmctl-database` are not published packages).

### Testing

- *(bbm)* assert graceful degradation in the live-API tests ([#47](https://github.com/timrabl/bbmctl/pull/47))

### CLI (`bbmctl`)

- `history import` gained `--on-duplicate <error|skip|update>` (default `error`)
  to control what happens when an imported row's timestamp already exists.
  Re-importing a file previously duplicated every row silently
  ([#46](https://github.com/timrabl/bbmctl/pull/46)).

### Dependencies

- Moved `sea-orm` from the `2.0.0-rc.38` release candidate to the stable
  **2.0.0** release, and dropped the exact version pin
  ([#45](https://github.com/timrabl/bbmctl/pull/45)).

### CI / Release

- release-plz now creates releases with a PAT, so a release fires the binary,
  `.deb`, and Homebrew workflows on its own instead of needing a manual trigger
  ([#43](https://github.com/timrabl/bbmctl/pull/43)).
- The Homebrew formula is now bumped by a deterministic step that rewrites the
  version and per-platform checksums, replacing an action that could not handle
  a multi-platform binary formula and failed on every release
  ([#44](https://github.com/timrabl/bbmctl/pull/44)).

## [0.1.2](https://github.com/timrabl/bbmctl/compare/v0.1.1...v0.1.2) - 2026-07-20

### Bug Fixes

- share one version across all workspace crates ([#37](https://github.com/timrabl/bbmctl/pull/37))

The `bbm` entry above is what is published to crates.io. The `bbmctl` binaries
attached to this release also contain the following, which release-plz does not
track because `bbmctl` and `bbmctl-database` are not published packages.

This is the first release whose binary and `.deb` report the correct version:
v0.1.1 shipped artifacts that identified themselves as `0.1.0` ([#37](https://github.com/timrabl/bbmctl/pull/37)).

#### CLI (`bbmctl`)

- The Prometheus `speedtest_measurements_total` counter was renamed to the
  `speedtest_measurements` gauge, since `history purge`/`delete` reduce it and
  a decreasing counter reads as a reset; non-finite samples now render as
  `+Inf`/`-Inf`/`NaN` instead of the `inf`/`NaN` that made a scrape
  unparseable ([#40](https://github.com/timrabl/bbmctl/pull/40))
- Manual Homebrew formula bumps are now possible via `workflow_dispatch`
  ([#38](https://github.com/timrabl/bbmctl/pull/38))

#### Storage

- `campaign record` now runs its measurement insert and completion update in a
  single transaction, and a partial unique index enforces at most one active
  campaign in the database; `settings` writes are now an atomic upsert
  ([#41](https://github.com/timrabl/bbmctl/pull/41))

> **Prometheus users:** `speedtest_measurements_total` is renamed to
> `speedtest_measurements`. Update any dashboards or alerts that reference it.

## [0.1.1](https://github.com/timrabl/bbmctl/compare/v0.1.0...v0.1.1) - 2026-07-20

### Bug Fixes

- *(bbm)* correct speed measurement, retries, and contract comparison ([#24](https://github.com/timrabl/bbmctl/pull/24))

The `bbm` library entries above are what is published to crates.io. The
`bbmctl` binaries attached to this release also contain the following, which
release-plz does not track because `bbmctl` and `bbmctl-database` are not
published packages.

#### Library (`bbm`)

- Download measurements discarded every byte of a request cut off by the
  deadline, so a payload larger than one measurement window reported
  `0.00 Mbit/s` as a success
- Upload measurements credited a full chunk for rejected requests, so a peer
  answering `405` reported multi-gigabit throughput
- A measurement that transferred nothing returned success instead of an error
- 5xx responses were never retried; the retry policy's status branch was
  unreachable
- Non-JSON responses could panic when the 200-byte preview cut a multi-byte
  character
- Retry backoff collapsed to zero at high attempt counts, and is now capped
- Latency measurement had no connect timeout and aborted on a single lost
  sample
- The API client had no request or connect timeout
- Unparseable contract thresholds were reported as met, which could show a
  line as `ALL PASS` against terms that were never checked

#### CLI (`bbmctl`)

- `test --every` could not be combined with `--provider`
- Scheduled tests drifted by the duration of every run, aborted on any
  transient error, and ignored Ctrl+C while a test was running
- `provider switch` had no effect on any other command
- The Prometheus exporter bound `0.0.0.0` without authentication, and a single
  half-open connection blocked every scrape; it now defaults to `127.0.0.1`
  and serves each connection independently
- `history export` and `history list -f csv` emitted incompatible schemas, and
  `list -f csv` produced structurally invalid output that aborted mid-write
- CSV import ignored the header row, accepted invalid timestamps that
  permanently broke `history summary` and the exporter, and was not atomic
- CSV export did not neutralise spreadsheet formula prefixes
- `compare` required `--provider` even when a provider was configured
- The `streams` and `duration` config keys were parsed but never applied
- A misplaced or misspelled config key was silently ignored

#### Storage

- A single unparseable timestamp permanently broke `history summary` and the
  metrics exporter
- Two timestamp formats were written to the same column, so lexicographic
  ordering did not match chronological order; existing rows are normalised by
  migration
- Added the missing indexes on `measurements.timestamp`,
  `measurements.campaign_id`, and `campaigns.status`

## [0.1.0] - 2026-04-03

### Features

- Async HTTP client for the Breitbandmessung API (providers, plans, speeds, statistics)
- Tower retry middleware with exponential backoff
- Concurrent speed test runner with configurable streams (FuturesUnordered)
- Plan comparison with threshold checking
- SQLite storage via SeaORM with schema versioning
- Measurement campaigns following BNetzA Nachweisverfahren protocol
- CLI with table, JSON, YAML, CSV output formats
- Config file support with named profiles
- Shell completions (bash, zsh, fish, powershell, elvish)
- Prometheus metrics exporter
- Import/export measurements as CSV
- Speed trend sparkline charts
- Scheduled recurring tests
- Human-readable speed display with auto unit detection
