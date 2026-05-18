# Copilot Instructions

## General Coding Guidelines

- Follow SOLID principles when they improve clarity, maintainability, or extensibility.
- Prefer simple, composable designs over tightly coupled implementations.
- Encourage the use of established design patterns when they fit the problem well.
- Do not force a design pattern where it adds unnecessary complexity.

## Design Pattern Naming

- When a class intentionally implements a design pattern, suffix the class name with the pattern name.
- Examples:
  - `TrendScenarioStrategy`
  - `MarketDataFactory`
  - `SignalObserver`
  - `ScenarioBuilder`
- Keep the suffix explicit and conventional so the role of the class is obvious from the name.
- Only use a pattern suffix when the type genuinely plays that pattern role.
- Do not attach a pattern suffix to generic domain models, DTOs, or utility types.

## Language-Specific Naming Guidance

### Rust

- Use idiomatic Rust naming for structs, enums, and traits.
- If a Rust type intentionally represents a design pattern, keep the pattern suffix in the type name.
- Prefer names such as:
  - `BinanceMarketDataAdapter`
  - `ScenarioScoringStrategy`
  - `CandleAggregationFactory`
  - `SignalObserver`
- Traits should describe behavior clearly and may also use a pattern suffix when appropriate, such as `ScenarioStrategy` or `MarketDataRepository`.
- Avoid artificial object-oriented layering that fights idiomatic Rust.

### Backend and Frontend Application Types

- Keep naming aligned with actual responsibility.
- Use domain-oriented names for core business types.
- Reserve pattern suffixes for types that intentionally encapsulate a pattern role.
- Examples:
  - `ScenarioEngine` for a domain service that coordinates scenario generation
  - `ScenarioScoringStrategy` for interchangeable scoring behavior
  - `ExchangeClientAdapter` for source-specific integration logic
  - `AlertFactory` for constructing alert payloads from internal signals
- Avoid vague names such as `Manager`, `Helper`, or `Processor` unless the responsibility is truly broad and well-justified.

## Commit Conventions

- Commit messages must follow the Conventional Commits specification.
- Prefer formats such as:
  - `feat: add BTC regime classifier`
  - `fix: handle stale websocket reconnect`
  - `docs: update data acquisition plan`
  - `refactor: extract scenario scoring strategy`

## Implementation Bias

- Favor readable, testable abstractions.
- Separate data acquisition, feature computation, scenario scoring, and presentation concerns.
- Prefer dependency inversion and interface-driven design where it meaningfully reduces coupling.