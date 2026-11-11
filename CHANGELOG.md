# Changelog

All notable changes to this project are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.3.2] - 2026-11-11

_No user-facing changes._

## [0.3.1] - 2026-11-08

### Fixed
- handle tray clicks directly on macOS 27 so the popover opens (#41)



## [0.3.0] - 2026-11-07

### Fixed
- fall back to weekly % when session window is absent and fix tray popover on macOS 27 (#39)
- price Claude Fable 5 / Mythos 5 and backfill zero-cost events (schema v7) (#38)



## [0.2.6] - 2026-07-14

### Fixed
- backfill persisted cost for Sonnet 5 (schema v6) (#36)



## [0.2.5] - 2026-07-14

### Fixed
- count cost for Sonnet 5 and future major model versions (#34)



## [0.2.4] - 2026-07-12

### Added
- surface live usage/limits panel in dashboard too (#32)
- add configurable menu-bar usage badge (#31)



## [0.2.3] - 2026-07-06

### Added
- refresh usage limits on popover open and handle 429 gracefully (#28)
- add group option to dashboard series segmentation (#27)



## [0.2.2] - 2026-07-06

### Added
- render changelog markdown in What's New and Changelog modals (#25)



## [0.2.1] - 2026-07-06

### Added
- recrea el landing según las capacidades reales de la app (#22)
- sistema de diseño de instrumento (DESIGN.md) y re-tema de la UI (#21)

### Fixed
- popover visible sobre Spaces en pantalla completa (#23)



## [0.2.0] - 2026-07-05

### Added
- vista de versión, modal What's New y changelog automático (#19)
- presupuestos de tokens por grupo de proyecto (#17)
- chequeo de actualizaciones en el frontend (#13)
- columna de nombre fija en la tabla del dashboard al scrollear (#11)
- límite semanal muestra horas restantes (<12h) o fecha de cierre (#10)
- brand app + menu-bar icons, fix dev Dock icon (#7)

### Fixed
- alinea todos los dominios a tokenwatch.gulloa.click (#16)
- gráfico muestra todos los buckets del rango, no solo los que tienen datos (#9)
