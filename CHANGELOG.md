# Changelog

All notable changes to Weatherspell are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While Weatherspell is in the `0.x` series, behaviour may change between minor
versions.

## [Unreleased]

### Added
- Find a place by name or postal code (Ctrl+L) and save it; the location
  last viewed comes back on the next launch.
- A forecast written in words from Open-Meteo data: right now, the rest of
  today by part of day, the coming week, sun and UV, and details such as
  humidity and pressure. Whole degrees, "kilometres an hour" rather than
  symbols, and the location's own times with yours in brackets when they
  differ. Units follow the Windows region setting.
- Ctrl+PageDown and Ctrl+PageUp jump between sections; F5 refreshes.

[Unreleased]: https://github.com/PlanetLinux98/weatherspell/commits/main
