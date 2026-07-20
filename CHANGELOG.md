# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1](https://github.com/timrabl/bbmctl/compare/v0.1.0...v0.1.1) - 2026-07-20

### Bug Fixes

- *(bbm)* correct speed measurement, retries, and contract comparison ([#24](https://github.com/timrabl/bbmctl/pull/24))

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
