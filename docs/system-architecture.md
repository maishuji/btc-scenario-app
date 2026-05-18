# BTC Scenario App System Architecture

## Purpose

This document defines a first-pass system architecture for the BTC Scenario App MVP.

The architecture is designed to satisfy the existing product and data acquisition plans while preserving room for later expansion. It assumes:

- The product is BTC-focused for the MVP
- Data is gathered from free public APIs
- Rust should own the data and analytics core where it provides real leverage
- The MVP is a decision-support dashboard, not an automated trading system

## Architecture Goals

The system should be designed to:

- Ingest reliable BTC market data from public sources
- Normalize and store a canonical internal market dataset
- Compute market features and scenario inputs consistently
- Serve a fast dashboard experience for end users
- Support alerts and historical comparison with minimal redesign later
- Keep responsibilities separated so each layer remains understandable and testable

## High-Level Architecture

The recommended MVP architecture has four main layers:

1. External data sources
2. Rust market intelligence backend
3. Persistent storage
4. User-facing application layer

At a high level:

- Exchange and reference APIs provide raw BTC market data
- A Rust service ingests and normalizes that data
- The Rust service computes candles, features, regimes, and scenario snapshots
- The processed outputs are stored in a database
- An application API exposes those outputs to the frontend
- The frontend renders dashboards, charts, scenarios, and alerts

## Recommended MVP Components

### 1. External Data Sources

These are outside the system boundary.

MVP sources:

- Binance public API as the primary live source
- Kraken public API as a validation or failover source
- CoinGecko as a reference and metadata source

Responsibilities:

- Historical candle retrieval
- Live ticker or trade streaming
- Reference price sanity checks

### 2. Rust Ingestion Service

This should be the first core service implemented in Rust.

Primary responsibility:

- Connect to external sources and turn raw exchange payloads into normalized internal market events

Recommended responsibilities:

- REST backfill for historical candles
- WebSocket consumption for live updates
- Retry and reconnect handling
- Source health tracking
- Symbol normalization
- Timestamp normalization
- Validation of inbound numeric data

Suggested internal types and roles:

- `BinanceMarketDataAdapter`
- `KrakenMarketDataAdapter`
- `CoinGeckoMarketDataAdapter`
- `MarketEventNormalizer`
- `SourceHealthMonitor`

This service should avoid business-facing scenario language. Its output should be normalized market data, not UI-ready narratives.

### 3. Candle Aggregation and Timeframe Service

This can live inside the Rust backend as a dedicated module or service boundary.

Primary responsibility:

- Build canonical 1m candles and derive higher timeframes consistently

Responsibilities:

- Finalize 1m OHLCV candles
- Derive 5m, 15m, 1h, 4h, 1d, and 1w candles
- Detect missing intervals
- Rebuild derived candles when reconciliation identifies gaps

Suggested roles:

- `CandleAggregationStrategy`
- `TimeframeDerivationEngine`
- `CandleGapDetector`

The key architectural decision here is that higher timeframes should be computed internally instead of relying on multiple vendor-specific interval feeds.

### 4. Feature Computation Service

This should also be Rust-owned in the MVP.

Primary responsibility:

- Convert canonical candle data into stable feature snapshots for scenario logic

Initial feature areas:

- Trend direction
- Momentum state
- Volatility state
- Volume confirmation
- Distance to support and resistance
- Strength of key level reactions

Suggested roles:

- `TrendFeatureStrategy`
- `MomentumFeatureStrategy`
- `VolatilityFeatureStrategy`
- `LevelDetectionStrategy`
- `FeatureSnapshotBuilder`

This layer should remain deterministic and explainable.

### 5. Regime and Scenario Engine

This is the core product logic layer.

Primary responsibility:

- Turn feature snapshots into regime labels and bull, base, and bear scenarios

Responsibilities:

- Compute regime state
- Score scenario probabilities or confidence levels
- Assign trigger and invalidation levels
- Generate structured explanation inputs
- Persist scenario snapshots for later comparison

Suggested roles:

- `MarketRegimeStrategy`
- `ScenarioScoringStrategy`
- `ScenarioExplanationBuilder`
- `ScenarioSnapshotFactory`

For the MVP, this engine should be rule-based first. Model-assisted scoring can be added later if needed.

### 6. Alerting Service

This can be implemented as a separate backend process or a module inside the application API, depending on deployment simplicity.

Primary responsibility:

- Detect meaningful changes in scenario state and produce user-facing alerts

Initial alert conditions:

- Regime changes
- Trigger level crossings
- Invalidation level crossings
- Material scenario score changes

Suggested roles:

- `AlertPolicyStrategy`
- `AlertFactory`
- `AlertDispatcher`

For the MVP, delivery can remain simple, such as in-app notifications and stored alert history.

### 7. Application API Layer

This layer serves processed data to the frontend.

Its implementation language is still open, but the design should assume it is a thin application-facing layer on top of the Rust-generated data.

Primary responsibility:

- Expose queryable, UI-ready endpoints without duplicating core analytics logic

Responsibilities:

- Serve current market overview
- Serve chart data for selected timeframes
- Serve scenario cards
- Serve explanation text or explanation inputs
- Serve historical scenario snapshots
- Serve alert state

This layer should not recompute the core market logic unless absolutely necessary.

### 8. Frontend Application

This is the user-facing dashboard.

Primary responsibility:

- Present BTC market state, scenarios, key levels, and alerts in a clear workflow

Responsibilities:

- Dashboard rendering
- Timeframe selection
- Chart overlays for levels and scenarios
- Scenario card presentation
- Alert views
- Historical analog display

The frontend should consume prepared data rather than reconstructing analytics client-side.

## Recommended Deployment Shape

The MVP should avoid an overly fragmented microservice architecture.

Recommended first deployment shape:

- One Rust backend process for ingestion, normalization, candle building, feature computation, and scenario generation
- One application API process if needed for frontend-specific delivery concerns
- One database for persistence
- One frontend application

This gives clean boundaries without creating operational overhead too early.

## Storage Architecture

The storage layer should support both live reads and historical analysis.

Recommended logical storage groups:

- Instrument metadata
- Canonical candles
- Live price snapshots
- Feature snapshots
- Regime snapshots
- Scenario snapshots
- Alert records
- Source health records

The system should treat normalized candle and feature data as reusable system assets, not temporary UI artifacts.

## Suggested Data Flow

The end-to-end flow for the MVP should be:

1. Pull historical candles from Binance through REST.
2. Open live WebSocket streams from Binance.
3. Normalize external events into internal market records.
4. Reconcile live state periodically using REST snapshots.
5. Build canonical 1m candles.
6. Derive higher timeframe candles.
7. Compute feature snapshots.
8. Determine regime and scenario states.
9. Persist scenario and feature snapshots.
10. Expose results through the application API.
11. Render the dashboard and evaluate alerts.

This is the main operating path the team should optimize around.

## Boundaries and Responsibilities

The most important architecture boundary is between market intelligence computation and product delivery.

Rust-owned domain boundary:

- Source connectivity
- Market normalization
- Candle generation
- Feature computation
- Regime classification
- Scenario scoring

Application-facing boundary:

- Query shaping
- Authentication if introduced later
- User preferences
- Alert presentation
- Frontend delivery

This separation keeps the analytical core portable and easier to test.

## Recommended Design Principles

The architecture should follow these principles:

- Keep market intelligence deterministic and explainable in the MVP
- Separate ingestion from analytics from presentation
- Prefer composition over tightly coupled inheritance trees
- Use pattern-based abstractions only when they reduce coupling or improve replaceability
- Keep the API layer thin where possible
- Avoid storing redundant derived data unless it provides clear read-performance value

## Suggested Pattern Usage

The project should use design patterns deliberately rather than ceremonially.

Patterns that fit this architecture well:

- Adapter for exchange-specific connectors
- Strategy for interchangeable feature or scoring logic
- Factory for snapshot or alert construction
- Observer for internal event-driven alert triggering if needed
- Repository for persistence abstractions if the codebase grows enough to justify it

Examples of appropriate naming:

- `BinanceMarketDataAdapter`
- `ScenarioScoringStrategy`
- `FeatureSnapshotFactory`
- `AlertObserver`

## Rule-Based vs Model-Based Decision

The MVP should start with rule-based regime and scenario logic.

Reasons:

- Easier to validate
- Easier to explain to users
- Better aligned with limited free public data
- Lower operational and modeling complexity

Model-assisted logic can be added after the rule-based system proves useful and the data foundation is stable.

## Historical Comparison Architecture

Historical comparison should be designed as an extension of the same feature and scenario snapshot model.

Recommended approach:

- Persist feature snapshots over time
- Persist regime and scenario outputs over time
- Add a later service or module that compares current feature states to historical feature states

This avoids building a separate historical system too early.

## Non-Goals for MVP Architecture

The first architecture should explicitly avoid:

- Trade execution infrastructure
- Portfolio accounting
- Full order book ingestion
- Large-scale event streaming infrastructure
- Heavy ML serving systems
- Multi-tenant enterprise complexity
- Premature microservices

These concerns would add significant complexity before the core product is validated.

## Open Decisions

These architecture decisions remain open:

1. Whether the application API should also be written in Rust or in another language
2. Which database should back the MVP storage layer
3. Whether alert dispatching should run inside the analytics backend or as a separate worker
4. Whether historical analog search belongs in the first release or the second
5. How much raw source data should be retained alongside normalized records

## Recommended Next Step

The next architecture activity should be to define:

- The concrete service boundaries
- The first database schema
- The external API endpoint matrix
- The initial Rust crate or module layout

Those decisions will turn this architecture into an implementation-ready technical plan.