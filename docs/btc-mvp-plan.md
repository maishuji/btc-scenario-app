# BTC Scenario App MVP Plan

## Goal

Build a BTC-focused application that helps users analyze the current market regime, understand the most likely next scenarios, and monitor the levels or signals that would confirm or invalidate those scenarios.

The MVP is intentionally centered on decision support, not automated trading.

## Core Product Promise

The app should help answer three questions clearly:

1. What market regime is BTC in right now?
2. What are the most likely next scenarios over key timeframes?
3. What evidence would strengthen or weaken each scenario?

## Target MVP User

The initial user is a discretionary BTC-focused trader or market observer who wants structured analysis and scenario framing rather than raw charts alone.

## MVP Scope

### 1. BTC Market Overview

Provide a compact dashboard for the current BTC state.

Must include:

- Live BTC price
- 24h change
- Volume summary
- Volatility summary
- Selected timeframes: 4h, 1d, 1w
- Current market regime label
- Short written market summary

Initial regime labels:

- Uptrend
- Downtrend
- Range
- High-volatility transition

### 2. Charting and Key Levels

Show the BTC price in a way that supports scenario analysis.

Must include:

- Main BTC price chart
- Support zones
- Resistance zones
- Recent swing highs and lows
- Volume overlay

Optional in v1 if easy:

- A small set of moving averages for context

### 3. Scenario Engine

Generate three forward-looking scenario cards:

- Bull case
- Base case
- Bear case

Each scenario should include:

- Probability or confidence score
- Trigger conditions
- Invalidation level
- Expected direction
- Time horizon
- Short explanation in plain language

### 4. Signal Inputs for v1

Keep the first version grounded in a limited set of interpretable inputs.

Recommended v1 inputs:

- Price structure
- Momentum
- Volume behavior
- Volatility regime
- Trend strength
- Reactions around key levels

These inputs should be enough to produce useful scenario analysis without overloading the MVP.

### 5. Scenario Explanations

Every scenario should explain why it exists in user-facing language.

Example:

"Bull scenario strengthened because BTC reclaimed resistance with improving momentum and stable volatility."

This is a core differentiator and should be treated as first-class product behavior, not an afterthought.

### 6. Alerts

Users should be able to track meaningful changes without constantly watching the screen.

Initial alert types:

- Scenario probability changes materially
- BTC crosses a trigger level
- BTC crosses an invalidation level
- Market regime changes

### 7. Historical Validation

Show a lightweight historical analog section to improve trust.

Initial version should show:

- A small set of recent similar setups
- Date of each setup
- Regime at that time
- What happened next over the selected horizon

This does not need to be a full backtesting system in the MVP.

## Out of Scope for MVP

To keep the first release focused, do not include:

- Automated trade execution
- Portfolio management
- Multi-asset coverage beyond BTC
- Large indicator library
- User scripting or strategy builder
- Full order book analytics
- Deep on-chain analytics
- Deep derivatives analytics
- Complex AI chat assistant

These can be revisited after the core BTC scenario workflow is working well.

## Recommended User Flow

1. User opens the app.
2. User sees the BTC regime and a short summary.
3. User reviews bull, base, and bear scenarios.
4. User checks the charted trigger and invalidation levels.
5. User reads why the scenario mix shifted.
6. User sets alerts for levels or regime changes.
7. User optionally checks similar past setups.

## Product Logic for v1

The initial system flow should be:

1. Ingest BTC market data.
2. Compute a small feature set.
3. Determine the current market regime.
4. Score bull, base, and bear scenarios.
5. Generate plain-language explanations.
6. Surface the results in the dashboard and alerts.

### Initial Feature Set

The first feature set should remain small and interpretable:

- Trend direction
- Momentum state
- Volatility state
- Volume confirmation
- Distance to important levels

## Delivery Priorities

### Phase 1

- BTC price ingestion
- Timeframe views
- Regime detection
- Support and resistance mapping
- Scenario cards
- Basic explanation layer

### Phase 2

- Alerts
- Historical analog view
- Better confidence scoring
- Scenario change history

### Phase 3

- Additional data layers such as derivatives, sentiment, or macro events
- More advanced explainability
- Personalization and watchlist behavior

## Rust Considerations

Rust should be used where it creates clear technical leverage rather than being forced into every layer.

The most promising areas for Rust are:

- Data ingestion services
- Market feature computation
- Scenario scoring engine
- Historical similarity engine
- Performance-sensitive backend services

Open architecture questions to resolve later:

- Whether Rust should own only the analytics engine or also the API layer
- Whether v1 should be rule-based only or partially model-based
- Whether derivatives data should be included in the first usable release

## Immediate Next Decisions

The next planning discussion should settle:

1. Dashboard-first vs assistant-first product direction
2. Technical-data-only vs technical-plus-derivatives MVP
3. Rule-based vs model-assisted scenario generation for v1
4. Which parts of the stack should be implemented in Rust first
5. The intended primary user: casual, discretionary trader, or research-oriented power user

## Working Definition of Success

The MVP is successful if a user can open the app and quickly understand:

- The current BTC regime
- The most likely near-term scenarios
- The key levels and signals to monitor next
- Why the app believes those scenarios matter