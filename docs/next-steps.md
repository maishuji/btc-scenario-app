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
- Binance source health events are persisted and exposed with dynamic freshness status.
- Deribit BTC perpetual derivatives snapshots are parsed, persisted, and exposed as a read-only API view.
- The frontend shows source status and feed age alongside the market overview.
- Local development can start the API and frontend together with `npm run dev` from the repo root.
- The API currently exposes:
  - `GET /api/market-overview`
  - `GET /api/candles?timeframe=1m&limit=...`
  - `GET /api/scenario-history?timeframe=1m&limit=...`
  - `GET /api/alerts?limit=...`
  - `GET /api/source-health`
  - `GET /api/derivatives`

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

### 1. Make Derivatives Context Visible in the Dashboard

Goal:

- Show the latest Deribit index price, mark price, open interest, and funding rate alongside the BTC overview.

Why next:

- The backend now persists a read-only derivatives snapshot, but the frontend does not consume it yet.
- A small derivatives panel adds context without changing the canonical spot workflow or introducing trading behavior.

Recommended scope:

- Load `/api/derivatives` independently from the market overview.
- Preserve the last successful derivatives panel when refreshes fail.
- Show a compact unavailable or stale state from `/api/source-health`.

### 2. Add Secondary Source Validation

Goal:

- Compare Binance against Kraken and make source divergence visible.

Why next:

- Binance health is now persisted and exposed, but Kraken and CoinGecko remain scaffolding.
- A second exchange can validate the primary feed without changing the canonical BTC workflow.

Recommended scope:

- Pull a lightweight Kraken reference price during the periodic sync.
- Record divergence and validation status in source health events.
- Keep Binance as the primary source until validation shows it is stale or divergent.

Suggested first version:

- Keep the validation path read-only and scoped to BTC spot.

### 3. Harden Partial Failures

Goal:

- Keep the last good dashboard snapshot when individual refreshes fail.

Why next:

- Source health now makes degraded data visible, so the UI should preserve useful context during outages.

Possible directions:

- Load dashboard endpoints independently.
- Preserve the last successful snapshot while showing a degraded banner.

### 4. Enrich Alert Metadata

Goal:

- Make alert interactions more explicit and less dependent on timestamp matching.

Why later:

- The current alert tape is already useful for local iteration.
- Timeframe expansion will reveal whether alert contracts need extra fields.

Recommended scope:

- Persist explicit linkage to scenario snapshots in API responses.
- Return richer alert metadata for frontend filtering and chart focus.

## Suggested Tomorrow Starting Point

If the goal is steady MVP progress, start by adding the derivatives panel to the dashboard.

Concrete next task:

1. Load the latest derivatives snapshot from `/api/derivatives`.
2. Add a compact derivatives context panel to the dashboard.
3. Keep derivatives refreshes independent from the primary market overview.
4. Validate with the frontend build, crate-scoped tests, and `cargo test --workspace` when a Rust 2024-compatible toolchain is available.

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
