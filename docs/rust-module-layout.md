# BTC Scenario App Rust Module Layout

## Purpose

This document defines the recommended Rust crate and module layout for the BTC Scenario App MVP.

The goal is to turn the system architecture into an implementation-ready Rust structure that keeps ingestion, normalization, analytics, and persistence responsibilities clear.

## Design Goals

The Rust codebase should be structured to:

- Keep market intelligence logic isolated from UI delivery concerns
- Separate exchange-specific adapters from core domain logic
- Make feature and scenario logic easy to test independently
- Support iterative growth without forcing early over-abstraction
- Respect the project naming rule for intentional design-pattern roles

## Recommended Rust Ownership

For the MVP, Rust should own:

- Source adapters
- Market event normalization
- Candle aggregation
- Feature computation
- Regime classification
- Scenario scoring
- Analytics persistence orchestration

Rust does not need to own the frontend. It may or may not own the application API layer in the first release.

## Recommended Repository Shape

The Rust implementation should start with a workspace layout that leaves room for future separation without requiring multiple deployable services immediately.

Suggested top-level shape:

```text
rust/
  Cargo.toml
  crates/
    market-intelligence-app/
    market-data-core/
    market-data-infrastructure/
    scenario-core/
    persistence-core/
```

This structure allows the project to keep one deployable application initially while separating reusable domain concerns into focused crates.

## Recommended Crates

### `market-intelligence-app`

Type:

- Binary crate

Purpose:

- Compose the runtime pieces of the MVP backend

Responsibilities:

- Application startup
- Configuration loading
- Dependency wiring
- Scheduler or task bootstrap
- Background worker initialization
- Graceful shutdown

This crate should stay thin and orchestration-focused.

Suggested modules:

- `config`
- `bootstrap`
- `runtime`
- `tasks`

### `market-data-core`

Type:

- Library crate

Purpose:

- Define the core market domain used across ingestion and analytics

Responsibilities:

- Canonical market types
- Candle types
- Timeframe types
- Instrument identifiers
- Source identifiers
- Validation helpers for domain-safe construction

Suggested modules:

- `instrument`
- `source`
- `timeframe`
- `candle`
- `snapshot`
- `value_objects`

This crate should not depend on exchange-specific transport logic.

### `market-data-infrastructure`

Type:

- Library crate

Purpose:

- Implement external exchange connectivity and normalization into core market types

Responsibilities:

- REST clients
- WebSocket clients
- Source payload parsing
- Symbol mapping
- Timestamp normalization
- Source health monitoring

Suggested modules:

- `adapters`
- `clients`
- `normalization`
- `health`

Suggested `adapters` submodules:

- `binance`
- `kraken`
- `coingecko`

Suggested types:

- `BinanceMarketDataAdapter`
- `KrakenMarketDataAdapter`
- `CoinGeckoMarketDataAdapter`
- `MarketEventNormalizer`
- `SourceHealthMonitor`

### `scenario-core`

Type:

- Library crate

Purpose:

- Hold deterministic analytics logic for features, regimes, and scenarios

Responsibilities:

- Candle aggregation
- Derived timeframe generation
- Feature computation
- Regime classification
- Scenario scoring
- Explanation input generation

Suggested modules:

- `aggregation`
- `features`
- `regime`
- `scenario`
- `explanations`

Suggested types:

- `CandleAggregationStrategy`
- `TimeframeDerivationEngine`
- `TrendFeatureStrategy`
- `MomentumFeatureStrategy`
- `VolatilityFeatureStrategy`
- `LevelDetectionStrategy`
- `MarketRegimeStrategy`
- `ScenarioScoringStrategy`
- `ScenarioExplanationBuilder`

This crate should be the most heavily tested analytical layer in the MVP.

### `persistence-core`

Type:

- Library crate

Purpose:

- Provide persistence-facing abstractions and storage implementations for the MVP backend

Responsibilities:

- Repository interfaces where justified
- Database model mapping
- Insert and query orchestration
- Persistence transactions for analytics snapshots

Suggested modules:

- `models`
- `repositories`
- `queries`
- `writes`

Suggested types:

- `CandleRepository`
- `FeatureSnapshotRepository`
- `ScenarioSnapshotRepository`
- `AlertRepository`

This crate should avoid owning business logic that belongs in the scenario engine.

## Suggested Module Layout Inside the Binary Crate

If the MVP starts as a single binary first, the internal module layout should still mirror the domain boundaries.

Suggested shape:

```text
src/
  main.rs
  config/
  app/
  orchestration/
```

Possible responsibilities:

- `config`: environment parsing and settings
- `app`: top-level application assembly
- `orchestration`: startup flows, background jobs, and pipeline wiring

The binary should call into library crates for actual logic.

## Dependency Direction

Dependency flow should remain one-directional where possible.

Recommended dependency direction:

- `market-intelligence-app` depends on all lower crates
- `market-data-infrastructure` depends on `market-data-core`
- `scenario-core` depends on `market-data-core`
- `persistence-core` depends on `market-data-core` and selected shared types
- `market-data-core` depends on no application-specific crates

Avoid reverse dependencies from the core crates into infrastructure or the application layer.

## Suggested Internal Boundaries

The most important internal separation is between these concerns:

1. External source handling
2. Canonical domain modeling
3. Analytical computation
4. Persistence
5. Application orchestration

This allows the team to change a source adapter without destabilizing scenario logic, and change scenario logic without rewriting persistence transport code.

## Proposed Detailed Module Breakdown

### `market-data-core`

Suggested internal shape:

```text
src/
  lib.rs
  instrument.rs
  source.rs
  timeframe.rs
  candle.rs
  market_snapshot.rs
  value_objects/
    mod.rs
    price.rs
    volume.rs
    timestamp.rs
```

### `market-data-infrastructure`

Suggested internal shape:

```text
src/
  lib.rs
  adapters/
    mod.rs
    binance.rs
    kraken.rs
    coingecko.rs
  clients/
    mod.rs
    rest_client.rs
    websocket_client.rs
  normalization/
    mod.rs
    symbol_mapper.rs
    timestamp_normalizer.rs
    market_event_normalizer.rs
  health/
    mod.rs
    source_health_monitor.rs
```

### `scenario-core`

Suggested internal shape:

```text
src/
  lib.rs
  aggregation/
    mod.rs
    candle_aggregation_strategy.rs
    timeframe_derivation_engine.rs
    candle_gap_detector.rs
  features/
    mod.rs
    trend_feature_strategy.rs
    momentum_feature_strategy.rs
    volatility_feature_strategy.rs
    level_detection_strategy.rs
    feature_snapshot_builder.rs
  regime/
    mod.rs
    market_regime_strategy.rs
  scenario/
    mod.rs
    scenario_scoring_strategy.rs
    scenario_snapshot_factory.rs
  explanations/
    mod.rs
    scenario_explanation_builder.rs
```

### `persistence-core`

Suggested internal shape:

```text
src/
  lib.rs
  models/
    mod.rs
    candle_record.rs
    feature_snapshot_record.rs
    regime_snapshot_record.rs
    scenario_snapshot_record.rs
    alert_record.rs
  repositories/
    mod.rs
    candle_repository.rs
    feature_snapshot_repository.rs
    regime_snapshot_repository.rs
    scenario_snapshot_repository.rs
    alert_repository.rs
  queries/
    mod.rs
    latest_snapshot_queries.rs
  writes/
    mod.rs
    snapshot_write_service.rs
```

## Pattern Guidance

Pattern usage should remain deliberate.

Patterns that are justified here:

- Adapter for exchange-specific source integrations
- Strategy for replaceable analytical logic
- Factory for snapshot creation when object construction becomes non-trivial
- Repository when persistence boundaries need to be abstracted for tests or backend changes

Patterns that should be used carefully:

- Builder only when snapshot construction actually becomes complex
- Observer only when internal event-driven flows are clearer than direct orchestration

Patterns that should be avoided early:

- Deep inheritance trees
- Generic manager-style abstractions with weak responsibility
- Artificial service layers that only forward calls

## Testing Strategy by Crate

Each crate should have a different testing emphasis.

### `market-data-core`

- Unit tests for domain invariants
- Validation tests for value object parsing and conversion

### `market-data-infrastructure`

- Parsing tests against captured source payload examples
- Normalization tests for symbol and timestamp conversion
- Failure handling tests for reconnect and invalid payload behavior

### `scenario-core`

- Unit tests for feature calculations
- Rule tests for regime classification
- Scenario scoring tests with fixed candle inputs
- Regression tests for explanation inputs

### `persistence-core`

- Mapping tests between domain and storage records
- Repository tests against a real or disposable database where practical

### `market-intelligence-app`

- Startup wiring tests where practical
- End-to-end smoke tests for the application runtime

## Suggested MVP Build Sequence

Recommended implementation order:

1. `market-data-core`
2. `market-data-infrastructure`
3. `scenario-core`
4. `persistence-core`
5. `market-intelligence-app`

This sequence follows the logical dependency direction and helps keep the first iterations grounded.

## Open Decisions

These points still need to be resolved during implementation planning:

1. Whether the application API should live inside `market-intelligence-app` or as a separate crate
2. Whether persistence should begin with a minimal direct SQL layer before introducing repositories broadly
3. Whether source adapters should expose a shared trait immediately or only after two adapters are fully implemented
4. Whether scenario explanation generation should remain deterministic text assembly or evolve into a separate narrative component later

## Working Definition of Success

The Rust layout is successful if it:

- Keeps core market logic independent from source-specific code
- Makes the scenario engine easy to test in isolation
- Supports incremental implementation without premature fragmentation
- Reflects the architecture and naming conventions already defined for the project