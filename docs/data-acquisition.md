# BTC Scenario App Data Acquisition Plan

## Purpose

This document defines how the BTC Scenario App should gather the market data required for the MVP.

The guiding constraint is that the first version should rely on free public APIs. The system should be designed around a small number of reliable sources, clear normalization rules, and an ingestion pipeline that supports scenario analysis without unnecessary complexity.

## Data Acquisition Goals

The MVP data layer should support these product capabilities:

- Current BTC market overview
- Multi-timeframe trend analysis
- Support and resistance detection
- Scenario generation
- Alerting on trigger and invalidation levels
- Lightweight historical comparison

The data layer does not need to support automated execution, deep market microstructure analytics, or broad multi-asset coverage in the first release.

## MVP Data Requirements

The minimum required dataset for the MVP is:

- BTC spot price
- OHLCV candle data
- Volume data
- Live ticker updates
- Basic trade or price update stream for near-real-time analysis

This is sufficient for:

- Market regime detection
- Scenario scoring
- Volatility estimation
- Trigger and invalidation logic
- Alert generation

## Recommended Data Sources

### Primary Source

Use Binance public market data APIs as the primary source for BTC spot data.

Why:

- Strong liquidity for BTCUSDT
- Free public REST and WebSocket endpoints
- Broad community adoption
- Suitable for live updates and historical candle backfill

Primary symbol for v1:

- BTCUSDT

### Secondary Source

Use Kraken public market data APIs as a secondary reference and fallback.

Why:

- Free public access
- Independent exchange source for cross-checking price behavior
- Useful if the primary feed becomes unavailable or stale

Primary Kraken market for comparison:

- XBT/USD

### Reference Source

Use CoinGecko as a reference and fallback source for market metadata and sanity checks.

Why:

- Easy access to spot market summary data
- Helpful for reference pricing and broader market fields
- Useful as a backup source for non-streaming market context

CoinGecko should not be treated as the canonical real-time market feed for scenario logic.

### Optional Later Source

If derivatives data is introduced after the initial release, Deribit is the best first candidate.

Possible later use cases:

- Open interest context
- Funding or futures premium proxies
- Derivatives-based scenario refinement

Derivatives data is not required for the MVP.

## Recommended Source Hierarchy

For the first release, use this source order:

1. Binance as canonical live spot feed
2. Kraken for secondary validation or failover checks
3. CoinGecko for reference data and sanity checks

This gives the system a clear primary source while still allowing basic resilience.

## Data Collection Model

The ingestion pipeline should use a hybrid approach:

- REST for historical backfill
- WebSocket for live updates
- Periodic REST reconciliation for correctness and recovery

This pattern is preferred because:

- REST is simple for bootstrapping history
- WebSocket is better for fresh updates and alert responsiveness
- Periodic reconciliation helps detect dropped messages or stale streams

## Canonical Internal Dataset

The internal source of truth for analytics should be built around:

- 1 minute OHLCV candles
- Latest ticker snapshot
- Optional trade stream summaries if needed for future enhancements

All higher timeframes used by the app should be derived internally from the canonical lower-timeframe data when possible.

Preferred internally derived timeframes:

- 5m
- 15m
- 1h
- 4h
- 1d
- 1w

This improves consistency across the analytics stack and reduces dependency on vendor-specific interval behavior.

## Initial Ingestion Flow

The recommended ingestion sequence is:

1. Backfill recent 1m candles from the primary exchange using REST.
2. Subscribe to live ticker and or trade streams using WebSocket.
3. Build or update the active 1m candle in memory.
4. Persist finalized candles to storage.
5. Derive higher timeframe candles internally.
6. Compute features from the canonical time series.
7. Produce scenario inputs and alert inputs from those features.

This should be the baseline operational flow for the MVP.

## Normalization Rules

Because public APIs differ in format and naming, the ingestion layer must normalize all inbound data.

Normalization should cover:

- Symbol naming
- Timestamps in UTC
- Numeric field parsing
- Timeframe labeling
- Exchange source identification
- Missing value handling

Examples:

- Binance BTCUSDT and Kraken XBT/USD should map to a single internal BTC spot instrument identity
- All event timestamps should be stored in UTC with clear precision handling
- Numeric strings from exchange APIs should be converted into validated numeric types before storage

## Data Freshness and Reliability Checks

The ingestion system should not assume public APIs are always correct or available.

Minimum safeguards for v1:

- Track last successful update timestamp per source
- Detect stale WebSocket streams
- Retry failed REST requests with backoff
- Reconcile live state against periodic REST snapshots
- Mark source status as healthy, degraded, or unavailable
- Detect and log missing candle intervals

These checks are necessary if the product is going to make live scenario claims.

## Storage Requirements

The first version does not need a complex warehouse, but it does need a clean internal model.

Recommended logical entities:

- instruments
- candles
- live_price_snapshots
- feature_snapshots
- scenario_snapshots
- alerts
- source_health_events

Useful candle fields:

- instrument_id
- source
- timeframe
- open_time
- open
- high
- low
- close
- volume
- trade_count if available

Useful feature fields:

- trend_score
- momentum_score
- volatility_score
- volume_confirmation_score
- support_distance
- resistance_distance

Useful scenario fields:

- timeframe
- bull_probability
- base_probability
- bear_probability
- trigger_level
- invalidation_level
- explanation
- generated_at

## Rust Responsibilities

Since Rust is expected to be part of the stack, the data acquisition and analytics core is the best place to use it first.

Recommended Rust-owned responsibilities:

- Exchange connectors
- REST backfill workers
- WebSocket consumers
- Candle aggregation
- Data normalization
- Feature computation
- Scenario scoring inputs

This is a good fit for Rust because these components benefit from predictable performance, strong typing, and reliable long-running process behavior.

## API Selection Criteria

Each external source should be evaluated against these standards:

- Public availability without paid access for core endpoints
- Stable spot BTC coverage
- Historical candle availability
- Live stream support if used as a primary feed
- Reasonable rate limits for MVP scale
- Clear and usable documentation

If a source fails these criteria, it should not be part of the initial production path.

## Constraints of Free Public APIs

The system design must account for the limitations of free public APIs.

Expected constraints:

- Rate limits
- Temporary outages
- Message drops or socket disconnects
- Inconsistent symbol naming
- Inconsistent timestamp conventions
- Limited historical retention on some endpoints
- Possible usage restrictions for high-frequency or commercial traffic

These limits are manageable for the MVP if the app is built around a modest ingestion footprint and a clear primary source.

## Out of Scope for Data Acquisition v1

The following should not be part of the first implementation:

- Full order book depth capture
- On-chain analytics ingestion
- News scraping
- Social sentiment collection
- Macro event calendar ingestion
- Multi-asset ingestion beyond BTC spot

These can be added later if they become necessary for scenario quality.

## Recommended v1 Decision

The recommended data acquisition setup for the MVP is:

- Binance as the primary live spot data source
- Kraken as a secondary validation source
- CoinGecko as a reference and fallback source
- REST plus WebSocket hybrid ingestion
- 1m candles as the canonical internal base timeframe
- Internally derived higher timeframes for analytics
- Rust as the owner of ingestion, normalization, and feature preparation

## Open Questions

These questions still need to be resolved before implementation begins:

1. Whether Binance should be the only live source in v1, with Kraken used only for health checks
2. Whether the MVP should store raw trades or only normalized candles and snapshots
3. How much historical backfill is required for the first useful release
4. Whether historical analog matching should be supported in the first delivery phase or the second
5. Whether derivatives data should be introduced immediately after MVP or only after product validation

## Working Definition of Success

The data acquisition layer is successful if it can reliably provide:

- Fresh BTC market data
- Internally consistent candle series across timeframes
- Enough signal quality for regime detection and scenario generation
- Stable inputs for alerts and historical comparison features