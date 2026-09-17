# Agent Instructions

## Implementation workflow

- Work on one focused Phase 1 slice at a time.
- Keep each slice independently reviewable and testable.
- Run the narrowest relevant formatter, linter, type checker, and tests before committing.
- Check the staged diff for unrelated changes before every commit.

## Commit messages

- Use the Conventional Commits format: `<type>(<scope>): <imperative summary>`.
- Use a detailed commit body that explains the motivation, implementation details, and verification performed.
- Create one commit for each focused implementation step.
- Use the configured Git author identity for commits. The current repository author is Quentin Cartier.
- Do not amend or rewrite earlier commits unless explicitly requested.

## Phase 1 scope

Phase 1 targets the BTC market overview workflow:

- BTC market-data ingestion
- User-facing timeframe views
- Market-regime detection
- Support and resistance mapping

Keep the MVP focused on decision support. Do not add automated trading, portfolio management, or unrelated data sources as part of this phase.
