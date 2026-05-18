# BTC Scenario App API Endpoint Matrix

## Purpose

This document defines the initial external API endpoint matrix for the BTC Scenario App MVP.

It translates the data acquisition strategy into concrete upstream endpoints, symbols, and intended usage so implementation can start without re-deciding the feed surface on every task.

## Scope

The matrix is intentionally limited to the sources already chosen for the MVP:

- Binance as the primary live spot source
- Kraken as the secondary validation source
- CoinGecko as the reference and fallback source

The focus is BTC spot only.

## Canonical Instrument Mapping

Internal instrument identity:

- `BTC-USD-SPOT`

External symbol mapping:

- Binance: `BTCUSDT`
- Kraken: `BTC/USD` for WebSocket and `XBTUSD` for REST OHLC
- CoinGecko: `bitcoin` with `vs_currency=usd`

The ingestion layer should normalize these to the same internal instrument.

## Selection Rules

The MVP should use these rules when selecting endpoints:

- Prefer the simplest public endpoint that provides the required market fact
- Prefer REST for bootstrap and reconciliation
- Prefer WebSocket for live updates
- Avoid deeper market data endpoints that are not required for regime and scenario logic
- Avoid pulling overlapping datasets when one canonical path is enough

## Binance Endpoint Matrix

Binance is the primary source for historical backfill and live market updates.

### REST Endpoints

#### Historical Candles

- Endpoint: `GET /api/v3/klines`
- Base URL: `https://api.binance.com`
- Example query:
  - `symbol=BTCUSDT`
  - `interval=1m`
  - `limit=1000`
  - optional `startTime`
  - optional `endTime`
- MVP usage:
  - Bootstrap recent 1m candle history
  - Fill gaps detected after reconnects
  - Reconcile in-memory candle state
- Why selected:
  - It is the canonical historical candle endpoint for the MVP

#### Latest Price

- Endpoint: `GET /api/v3/ticker/price`
- Base URL: `https://api.binance.com`
- Example query:
  - `symbol=BTCUSDT`
- MVP usage:
  - Lightweight current price checks
  - Fast fallback for simple health validation

#### 24h Market Summary

- Endpoint: `GET /api/v3/ticker/24hr`
- Base URL: `https://api.binance.com`
- Example query:
  - `symbol=BTCUSDT`
  - optional `type=MINI`
- MVP usage:
  - Populate 24h change, high, low, and volume fields for the dashboard
  - Support occasional reconciliation of rolling market summary values

#### Aggregate Trades

- Endpoint: `GET /api/v3/aggTrades`
- Base URL: `https://api.binance.com`
- Example query:
  - `symbol=BTCUSDT`
  - optional `fromId`
  - optional `startTime`
  - optional `endTime`
  - optional `limit`
- MVP usage:
  - Optional backfill path if the team decides to maintain short-range trade summaries
- MVP decision:
  - Not required for the first delivery unless trade-derived features are introduced early

### WebSocket Endpoints

Base WebSocket URLs:

- `wss://stream.binance.com:9443`
- `wss://stream.binance.com:443`

Market-data-only alternative:

- `wss://data-stream.binance.vision`

#### 1m Kline Stream

- Stream name: `btcusdt@kline_1m`
- Path example:
  - `/ws/btcusdt@kline_1m`
- MVP usage:
  - Maintain the active 1m candle in near real time
  - Finalize candles on close
- Priority:
  - High

#### Mini Ticker Stream

- Stream name: `btcusdt@miniTicker`
- Path example:
  - `/ws/btcusdt@miniTicker`
- MVP usage:
  - Lightweight 24h rolling summary updates for the dashboard
- Priority:
  - Medium

#### Trade Stream

- Stream name: `btcusdt@trade`
- Path example:
  - `/ws/btcusdt@trade`
- MVP usage:
  - Optional if the team wants finer real-time price movement visibility than kline updates alone
- MVP decision:
  - Optional for the first implementation

#### Aggregate Trade Stream

- Stream name: `btcusdt@aggTrade`
- Path example:
  - `/ws/btcusdt@aggTrade`
- MVP usage:
  - Optional alternative to raw trade streaming when aggregated trade flow is preferred
- MVP decision:
  - Optional for the first implementation

### Binance Operational Notes

- Symbols in stream names must be lowercase
- A WebSocket connection is valid for 24 hours and should be rotated cleanly
- The server sends ping frames and expects pong responses
- Live streams should be reconciled periodically with REST snapshots

### Binance MVP Recommendation

Use these Binance feeds in the first implementation:

- `GET /api/v3/klines`
- `GET /api/v3/ticker/24hr`
- `GET /api/v3/ticker/price`
- `btcusdt@kline_1m`
- `btcusdt@miniTicker`

Start without order book or depth endpoints.

## Kraken Endpoint Matrix

Kraken acts as a secondary validation source and optional failover reference.

### REST Endpoints

#### OHLC Data

- Endpoint: `GET /0/public/OHLC`
- Base URL: `https://api.kraken.com`
- Example query:
  - `pair=XBTUSD`
  - `interval=1`
  - optional `since`
- MVP usage:
  - Cross-check Binance candle continuity
  - Build validation comparisons for price and timeframe behavior
- Important note:
  - Kraken returns up to 720 of the most recent OHLC entries
  - The last entry is the current, not-yet-committed timeframe

### WebSocket Endpoints

Base WebSocket URL:

- `wss://ws.kraken.com/v2`

#### Ticker Channel

- Channel: `ticker`
- Example subscription symbol:
  - `BTC/USD`
- Example event trigger:
  - `trades`
- MVP usage:
  - Secondary live validation of price and top-of-book state
  - Detect major divergence or primary-source staleness

### Kraken Operational Notes

- Kraken uses `XBT` in REST pair notation and `BTC/USD` in the documented WebSocket examples
- Kraken should not be the main MVP live feed unless primary-source issues force a fallback mode
- Kraken is best used as a secondary truth source, not as a second canonical history source

### Kraken MVP Recommendation

Use these Kraken feeds in the first implementation:

- `GET /0/public/OHLC`
- WebSocket `ticker` channel for `BTC/USD`

Do not mirror the full Binance ingestion footprint on Kraken in the MVP.

## CoinGecko Endpoint Matrix

CoinGecko is a reference and fallback source for market summary data.

### Reference Price Endpoint

- Endpoint: `GET /api/v3/simple/price`
- Base URL:
  - depends on the access tier in use
- Example query:
  - `ids=bitcoin`
  - `vs_currencies=usd`
  - `include_24hr_vol=true`
  - `include_24hr_change=true`
  - `include_last_updated_at=true`
- MVP usage:
  - Reference price sanity checks
  - Basic market-cap and 24h summary fallback

### Market Summary Endpoint

- Endpoint: `GET /api/v3/coins/markets`
- Base URL:
  - depends on the access tier in use
- Example query:
  - `vs_currency=usd`
  - `ids=bitcoin`
  - `price_change_percentage=24h,7d`
- MVP usage:
  - Market metadata and broader summary display fields if needed
  - Secondary reference values for UI support

### CoinGecko Operational Notes

- CoinGecko documentation emphasizes tiered access and API-key-based usage in the official reference docs
- Because the product goal is free public API usage, CoinGecko should be treated as optional reference infrastructure unless the chosen access tier is confirmed compatible with the MVP constraints
- CoinGecko should not be used as the canonical low-latency market feed for scenario generation

### CoinGecko MVP Recommendation

Use CoinGecko only for:

- Reference price checks
- Supplemental market metadata
- UI fallback values when exchange summaries are unavailable

Do not make CoinGecko a required dependency for the live analytics loop.

## Endpoint Priority Matrix

### Required for MVP Core

- Binance `GET /api/v3/klines`
- Binance `GET /api/v3/ticker/24hr`
- Binance `GET /api/v3/ticker/price`
- Binance `btcusdt@kline_1m`
- Binance `btcusdt@miniTicker`

### Recommended for MVP Reliability

- Kraken `GET /0/public/OHLC`
- Kraken WebSocket `ticker` channel for `BTC/USD`

### Optional for MVP Expansion

- Binance `GET /api/v3/aggTrades`
- Binance `btcusdt@trade`
- Binance `btcusdt@aggTrade`
- CoinGecko `GET /api/v3/simple/price`
- CoinGecko `GET /api/v3/coins/markets`

## Ingestion Mapping

The implementation should map each endpoint to a clear pipeline role.

### Historical Bootstrap

- Binance `GET /api/v3/klines`

### Live Candle Maintenance

- Binance `btcusdt@kline_1m`

### Live Dashboard Summary

- Binance `btcusdt@miniTicker`
- Binance `GET /api/v3/ticker/24hr` for periodic reconciliation

### Current Price Fallback

- Binance `GET /api/v3/ticker/price`
- CoinGecko `GET /api/v3/simple/price` if available under the chosen access plan

### Source Validation

- Kraken `GET /0/public/OHLC`
- Kraken WebSocket `ticker` channel

## Failure and Fallback Policy

The MVP should follow this behavior when upstream issues happen:

1. Prefer Binance as long as freshness checks remain healthy.
2. If Binance WebSocket freshness degrades, use Binance REST reconciliation immediately.
3. If Binance appears unavailable or stale beyond threshold, compare against Kraken.
4. Use CoinGecko only for coarse reference values, not for canonical live analytics.

## Implementation Notes

The first implementation should avoid unnecessary endpoint sprawl.

Recommended first cut:

- Build only the Binance kline and market-summary path
- Add Kraken validation after the primary flow is stable
- Add CoinGecko last, only if the product benefits from the extra reference data

This sequence keeps the first usable ingestion path small and testable.

## Open Decisions

These points still need confirmation during implementation:

1. Whether the MVP should subscribe to Binance trade streams in addition to kline and mini ticker streams
2. Whether Kraken validation should run continuously or only on scheduled reconciliation intervals
3. Whether CoinGecko access is acceptable under the final free-tier constraints for this project
4. Whether higher timeframe REST endpoints should ever be queried directly or always derived internally

## Working Definition of Success

The endpoint matrix is successful if it gives the implementation team:

- A single canonical upstream path for live BTC market data
- A secondary validation path that is clearly scoped
- A minimal, concrete set of endpoints to build first
- Enough specificity to wire adapters and tests without further API-selection debate