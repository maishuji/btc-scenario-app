# BTC Scenario App Database Schema

## Purpose

This document defines the first database schema for the BTC Scenario App MVP.

The schema is designed to support:

- Canonical BTC market data storage
- Feature and regime computation outputs
- Scenario snapshots for the dashboard
- Alert persistence
- Source health tracking
- Future historical comparison without major redesign

The schema is intentionally scoped to the MVP and should not be treated as a final enterprise design.

## Design Goals

The database design should:

- Preserve a clean canonical market dataset
- Separate raw market facts from derived analytics
- Support efficient reads for the dashboard and alerts
- Allow historical replay of features and scenarios
- Stay small enough for a simple MVP deployment

## Recommended Database Role

For the MVP, the database should act as the persistent store for normalized records and computed outputs.

It should store:

- Instruments and sources
- Canonical candles
- Live market snapshots
- Derived feature snapshots
- Regime snapshots
- Scenario snapshots
- Alert records
- Source health events

It should not be treated as the place where the frontend reconstructs analytics logic from scratch.

## Schema Overview

Recommended logical tables:

- `instruments`
- `data_sources`
- `candles`
- `live_price_snapshots`
- `feature_snapshots`
- `regime_snapshots`
- `scenario_snapshots`
- `alerts`
- `source_health_events`

Optional later tables:

- `raw_market_events`
- `historical_analog_matches`
- `user_alert_rules`

## Table Definitions

### `instruments`

Stores the internal canonical market identity used across sources.

Suggested columns:

- `id`
- `symbol`
- `base_asset`
- `quote_asset`
- `market_type`
- `is_active`
- `created_at`
- `updated_at`

Notes:

- For the MVP, this will likely contain a single BTC spot instrument.
- Example internal symbol: `BTC-USD-SPOT`.

### `data_sources`

Stores known upstream providers.

Suggested columns:

- `id`
- `name`
- `source_type`
- `is_primary`
- `is_active`
- `created_at`
- `updated_at`

Expected rows for MVP:

- Binance
- Kraken
- CoinGecko

### `candles`

Stores canonical OHLCV time series used by the analytics engine.

Suggested columns:

- `id`
- `instrument_id`
- `source_id`
- `timeframe`
- `open_time`
- `close_time`
- `open`
- `high`
- `low`
- `close`
- `volume`
- `trade_count`
- `is_final`
- `created_at`

Notes:

- The canonical base timeframe should be `1m`.
- Higher timeframes such as `5m`, `15m`, `1h`, `4h`, `1d`, and `1w` should be stored if derived candles materially improve query performance.
- If storage simplicity is preferred early on, only `1m` candles can be persisted first and higher timeframes can be derived on demand or in-memory.

Suggested uniqueness rule:

- Unique by `instrument_id`, `source_id`, `timeframe`, and `open_time`

### `live_price_snapshots`

Stores recent market state snapshots for fast dashboard reads.

Suggested columns:

- `id`
- `instrument_id`
- `source_id`
- `last_price`
- `price_change_24h`
- `volume_24h`
- `observed_at`
- `created_at`

Notes:

- This table is optimized for current-state views, not long-range historical analytics.
- Older entries can be pruned or retained with a short time horizon.

### `feature_snapshots`

Stores derived features used by regime classification and scenario scoring.

Suggested columns:

- `id`
- `instrument_id`
- `timeframe`
- `observed_at`
- `trend_score`
- `momentum_score`
- `volatility_score`
- `volume_confirmation_score`
- `support_distance`
- `resistance_distance`
- `level_reaction_score`
- `feature_version`
- `created_at`

Notes:

- This table is one of the core foundations for historical comparison.
- `feature_version` should track changes to feature logic over time.

Suggested uniqueness rule:

- Unique by `instrument_id`, `timeframe`, `observed_at`, and `feature_version`

### `regime_snapshots`

Stores classified market regime outputs.

Suggested columns:

- `id`
- `instrument_id`
- `timeframe`
- `observed_at`
- `regime_label`
- `regime_score`
- `regime_version`
- `created_at`

Expected initial regime labels:

- `uptrend`
- `downtrend`
- `range`
- `high_volatility_transition`

Notes:

- Keep regime outputs separate from feature outputs so they remain independently testable.

### `scenario_snapshots`

Stores the user-facing analytical output for bull, base, and bear scenarios.

Suggested columns:

- `id`
- `instrument_id`
- `timeframe`
- `observed_at`
- `bull_probability`
- `base_probability`
- `bear_probability`
- `trigger_level`
- `invalidation_level`
- `expected_direction`
- `explanation`
- `scenario_version`
- `created_at`

Notes:

- This is the primary analytical table for the dashboard.
- `expected_direction` can hold values such as `bullish`, `neutral`, or `bearish`.
- The explanation should be stored as generated output rather than rebuilt at render time.

Suggested uniqueness rule:

- Unique by `instrument_id`, `timeframe`, `observed_at`, and `scenario_version`

### `alerts`

Stores emitted alert records.

Suggested columns:

- `id`
- `instrument_id`
- `timeframe`
- `alert_type`
- `severity`
- `message`
- `triggered_at`
- `scenario_snapshot_id`
- `is_acknowledged`
- `created_at`

Initial alert types:

- `regime_changed`
- `trigger_crossed`
- `invalidation_crossed`
- `scenario_shifted`

Notes:

- This table stores system-generated alerts for the MVP.
- User-specific alert preferences can be added later without changing the core analytics schema.

### `source_health_events`

Stores availability and freshness events for upstream sources.

Suggested columns:

- `id`
- `source_id`
- `status`
- `message`
- `observed_at`
- `created_at`

Suggested statuses:

- `healthy`
- `degraded`
- `unavailable`

Notes:

- This table supports debugging and operational visibility.
- It is useful for explaining missing data or stale dashboards.

## Relationships

Primary relationships:

- `candles.instrument_id` references `instruments.id`
- `candles.source_id` references `data_sources.id`
- `live_price_snapshots.instrument_id` references `instruments.id`
- `live_price_snapshots.source_id` references `data_sources.id`
- `feature_snapshots.instrument_id` references `instruments.id`
- `regime_snapshots.instrument_id` references `instruments.id`
- `scenario_snapshots.instrument_id` references `instruments.id`
- `alerts.instrument_id` references `instruments.id`
- `alerts.scenario_snapshot_id` references `scenario_snapshots.id`
- `source_health_events.source_id` references `data_sources.id`

## Indexing Strategy

The MVP indexing strategy should optimize for time-series reads and latest-state lookups.

Recommended indexes:

- `candles` on `instrument_id`, `timeframe`, `open_time`
- `candles` on `source_id`, `timeframe`, `open_time`
- `feature_snapshots` on `instrument_id`, `timeframe`, `observed_at`
- `regime_snapshots` on `instrument_id`, `timeframe`, `observed_at`
- `scenario_snapshots` on `instrument_id`, `timeframe`, `observed_at`
- `alerts` on `instrument_id`, `triggered_at`
- `source_health_events` on `source_id`, `observed_at`

If the selected database supports descending indexes or clustered time-series layout, the most recent snapshot reads should be optimized for current dashboard queries.

## Data Retention Guidance

Retention should differ by table type.

Recommended initial approach:

- `candles`: retain full available history for the MVP if practical
- `live_price_snapshots`: short retention window or compaction strategy
- `feature_snapshots`: retain history for historical comparison
- `regime_snapshots`: retain history
- `scenario_snapshots`: retain history
- `alerts`: retain history
- `source_health_events`: retain a moderate operational window

If storage pressure becomes an issue, the first candidate for aggressive pruning should be `live_price_snapshots`.

## Versioning Strategy

Derived analytics should be versioned explicitly.

Recommended version fields:

- `feature_version`
- `regime_version`
- `scenario_version`

This makes it possible to:

- Compare output quality across logic changes
- Preserve historical interpretation correctness
- Avoid silent mixing of incompatible analytical outputs

## Minimal Query Patterns

The schema should support these core reads efficiently:

- Latest dashboard snapshot for BTC
- Recent candles for a selected timeframe
- Latest feature snapshot per timeframe
- Latest regime snapshot per timeframe
- Latest scenario snapshot per timeframe
- Recent alerts
- Historical scenario snapshots for comparison

## Suggested MVP Implementation Order

The schema can be implemented in this order:

1. `instruments`
2. `data_sources`
3. `candles`
4. `feature_snapshots`
5. `regime_snapshots`
6. `scenario_snapshots`
7. `alerts`
8. `source_health_events`
9. `live_price_snapshots`

This order follows the data pipeline from raw normalized storage toward user-facing outputs.

## Open Decisions

These points still need to be finalized before implementation:

1. Which database engine will back the MVP
2. Whether higher timeframe candles should be persisted or derived at read time initially
3. Whether raw inbound market events should be stored in v1 or ignored after normalization
4. Whether explanations should remain a single text field or be split into structured evidence fields
5. Whether alert history needs user-specific ownership in the first release

## Working Definition of Success

The schema is successful if it can:

- Preserve a clean canonical BTC dataset
- Support fast reads for the dashboard
- Store explainable analytical outputs over time
- Enable historical comparison without major schema changes
- Stay simple enough to implement safely in the MVP