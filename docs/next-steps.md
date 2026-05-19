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

## Recommended Next Order

The next work should stay focused on making the system usable by a frontend without widening into new data sources too early.

### 1. Build The First Frontend Shell

Goal:

- Create a usable dashboard that consumes the existing API.

Why next:

- The backend now exposes a minimally useful read surface for an initial UI.
- Alert reads now return real generated history instead of only seeded rows.
- Frontend work will quickly expose any remaining API shape issues.

Recommended scope:

- Show current market overview.
- Render a recent candle chart from `/api/candles`.
- Render scenario history from `/api/scenario-history`.
- Render alerts from `/api/alerts`.

Suggested first version:

- Start with a local dashboard shell and simple polling.
- Keep the visual contract thin until the UI proves what fields are missing.

### 2. Split Runtime Roles If Needed

Goal:

- Separate ingestion and API serving if running both in one binary becomes awkward.

Why later:

- The current `serve-api` mode is enough for local iteration.
- Process separation is useful, but it is not the highest product-value step right now.

Possible directions:

- Separate binaries in the same crate.
- Separate app crates for ingestion and API.

### 3. Add Higher Timeframes

- Expand from canonical 1m data into derived user-facing timeframes.

Why not first:

- The existing backend is still proving the 1m path.
- Higher timeframe derivation is more valuable after the frontend is visible.

Recommended first additions:

- `5m`
- `15m`
- `1h`

## Suggested Tomorrow Starting Point

If the goal is steady MVP progress, start with the frontend shell.

Concrete next task:

1. Add a frontend workspace or app shell.
2. Render market overview, candles, scenario history, and alerts.
3. Wire simple polling against the current API endpoints.
4. Adjust API response shapes only where the UI reveals real gaps.
5. Run frontend validation plus `cargo test --workspace`.

## Relevant Files To Open First Tomorrow

- `rust/crates/persistence-core/src/lib.rs`
- `rust/crates/market-intelligence-app/src/main.rs`
- `docs/api-endpoint-matrix.md`
- `docs/system-architecture.md`
- `docs/btc-mvp-plan.md`

## Notes To Keep In Mind

- Keep the API layer thin and query-oriented.
- Do not duplicate scenario logic inside handlers or the frontend.
- Reuse the existing persistence query/service pattern.
- Prefer narrow crate-scoped tests before full workspace validation.
- Keep commits split by slice and follow Conventional Commits.