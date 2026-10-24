# Changelog

All notable changes are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Project plan, cargo workspace scaffold, and open-source project files.
- HTTP request sending, collections, workspace variables, profiles, and Keychain secret management.

### Changed
- Request-file headers and query params now provide base values that profiles can override.
- Environment variables from the process are no longer used for request interpolation; missing environments and secrets now report actionable errors.
- `secret ls` reports referenced secret status in human-readable or JSON output.
