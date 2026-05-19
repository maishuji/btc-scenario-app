# Next Steps

This document captures the current implementation state and the recommended next sequence so work can resume quickly.

## Current Backend Status

The Rust workspace now covers the main backend MVP path for a BTC-first scenario app:

- Binance REST bootstrap ingestion is implemented.
- Binance WebSocket live sync is implemented.
- Canonical 1m candles and live market snapshots are persisted to SQLite.
- Feature, regime, and scenario snapshots are computed and persisted.
- Basic alerts are generated and persisted when regime or scenario direction changes.
- The application can read the latest market overview from persistence.
- A frontend dashboard shell renders market overview, chart context, scenario history, and alerts.
- Alerts and scenario history can drive chart projections in the frontend.
- Local development can start the API and frontend together with `npm run dev` from the repo root.
- The API currently exposes:
  - `GET /api/market-overview`
  - `GET /api/candles?timeframe=1m&limit=...`
  - `GET /api/scenario-history?timeframe=1m&limit=...`
  - `GET /api/alerts?limit=...`

Recent completed commits:

- `feat: persist analytics snapshots`
- `feat: persist runtime analytics snapshots`
- `feat: add market overview read path`
- `feat: expose market overview endpoint`
- `feat: expose market history endpoints`
- `feat: generate alerts from snapshot changes`
- `feat: add market dashboard shell`
- `feat: add scenario projection controls`
- `feat: link alerts to scenario projections`

## Recommended Next Order

The next work should stay focused on making the system usable by a frontend without widening into new data sources too early.

### 1. Add Higher Timeframes

Goal:

- Expand from canonical 1m data into derived user-facing timeframes.

Why next:

- The frontend now makes the single-timeframe limitation visible.
- Higher timeframe context is more valuable than more UI polish on only `1m` data.
- The backend already has a stable canonical candle path to derive from.

Recommended scope:

- Derive `5m`, `15m`, and `1h` candles from canonical `1m` data.
- Expose those timeframes through the existing candle and scenario endpoints.
- Let the frontend switch between supported timeframes.

Suggested first version:

- Start with `5m` and `15m` if `1h` widens the slice too much.
- Keep derivation inside the backend instead of rebuilding candles client-side.

### 2. Split Runtime Roles If Needed

Goal:

- Separate ingestion and API serving if running both in one binary becomes awkward.

Why later:

- The current `serve-api` mode is enough for local iteration.
- Process separation is useful, but it is not the highest product-value step right now.

Possible directions:

- Separate binaries in the same crate.
- Separate app crates for ingestion and API.

### 3. Enrich Alert Metadata

Goal:

- Make alert interactions more explicit and less dependent on timestamp matching.

Why after higher timeframes:

- The current alert tape is already useful for local iteration.
- Timeframe expansion will reveal whether alert contracts need extra fields.

Recommended scope:

- Persist explicit linkage to scenario snapshots in API responses.
- Return richer alert metadata for frontend filtering and chart focus.

## Suggested Tomorrow Starting Point

If the goal is steady MVP progress, start with higher timeframes.

Concrete next task:

1. Add backend candle derivation for `5m` and `15m`.
2. Expose those timeframes through the current API surface.
3. Add a frontend timeframe selector.
4. Validate with crate-scoped tests, `npm run build`, and `cargo test --workspace`.

## Relevant Files To Open First Tomorrow

- `rust/crates/persistence-core/src/lib.rs`
- `rust/crates/market-intelligence-app/src/main.rs`
- `rust/crates/market-data-core/src/lib.rs`
- `docs/api-endpoint-matrix.md`
- `docs/system-architecture.md`
- `docs/btc-mvp-plan.md`

## Notes To Keep In Mind

- Keep the API layer thin and query-oriented.
- Do not duplicate scenario logic inside handlers or the frontend.
- Reuse the existing persistence query/service pattern.
- Prefer narrow crate-scoped tests before full workspace validation.
- Keep commits split by slice and follow Conventional Commits.