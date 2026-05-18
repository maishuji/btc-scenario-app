use market_data_core::candle::Candle;
use scenario_core::{FeatureSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

pub mod models {
    use super::{Candle, FeatureSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

    #[derive(Debug, Clone, PartialEq)]
    pub struct CandleRecord {
        pub candle: Candle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FeatureSnapshotRecord {
        pub snapshot: FeatureSnapshot,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct RegimeSnapshotRecord {
        pub snapshot: MarketRegimeSnapshot,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ScenarioSnapshotRecord {
        pub snapshot: ScenarioSnapshot,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AlertRecord {
        pub instrument_id: String,
        pub alert_type: String,
        pub message: String,
        pub triggered_at_ms: i64,
    }
}

pub mod repositories {
    use super::{Candle, FeatureSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

    pub trait CandleRepository {
        type Error;

        fn save(&self, candle: &Candle) -> Result<(), Self::Error>;
    }

    pub trait FeatureSnapshotRepository {
        type Error;

        fn save(&self, snapshot: &FeatureSnapshot) -> Result<(), Self::Error>;
    }

    pub trait RegimeSnapshotRepository {
        type Error;

        fn save(&self, snapshot: &MarketRegimeSnapshot) -> Result<(), Self::Error>;
    }

    pub trait ScenarioSnapshotRepository {
        type Error;

        fn save(&self, snapshot: &ScenarioSnapshot) -> Result<(), Self::Error>;
    }
}

pub mod queries {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LatestSnapshotQueries {
        pub instrument_id: String,
        pub timeframe: String,
    }

    impl LatestSnapshotQueries {
        pub fn latest_scenario_for(instrument_id: impl Into<String>, timeframe: impl Into<String>) -> Self {
            Self {
                instrument_id: instrument_id.into(),
                timeframe: timeframe.into(),
            }
        }
    }
}

pub mod writes {
    use super::repositories::{FeatureSnapshotRepository, RegimeSnapshotRepository, ScenarioSnapshotRepository};
    use super::{FeatureSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct SnapshotWriteService;

    impl SnapshotWriteService {
        pub fn save_feature_snapshot<R>(self, repository: &R, snapshot: &FeatureSnapshot) -> Result<(), R::Error>
        where
            R: FeatureSnapshotRepository,
        {
            repository.save(snapshot)
        }

        pub fn save_regime_snapshot<R>(self, repository: &R, snapshot: &MarketRegimeSnapshot) -> Result<(), R::Error>
        where
            R: RegimeSnapshotRepository,
        {
            repository.save(snapshot)
        }

        pub fn save_scenario_snapshot<R>(self, repository: &R, snapshot: &ScenarioSnapshot) -> Result<(), R::Error>
        where
            R: ScenarioSnapshotRepository,
        {
            repository.save(snapshot)
        }
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    const INITIAL_MIGRATION: &str =
        include_str!("../../../../db/migrations/0001_initial_schema.sql");

    #[test]
    fn applies_initial_schema_migration() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite connection");
        connection
            .execute_batch(INITIAL_MIGRATION)
            .expect("migration should apply cleanly");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                    'instruments',
                    'data_sources',
                    'candles',
                    'live_price_snapshots',
                    'feature_snapshots',
                    'regime_snapshots',
                    'scenario_snapshots',
                    'alerts',
                    'source_health_events'
                )",
                [],
                |row| row.get(0),
            )
            .expect("table count query should succeed");

        assert_eq!(table_count, 9);
    }

    #[test]
    fn seeds_canonical_instrument_and_sources() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite connection");
        connection
            .execute_batch(INITIAL_MIGRATION)
            .expect("migration should apply cleanly");

        let instrument_symbol: String = connection
            .query_row(
                "SELECT symbol FROM instruments WHERE id = 'BTC-USD-SPOT'",
                [],
                |row| row.get(0),
            )
            .expect("seeded instrument should exist");
        let source_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM data_sources", [], |row| row.get(0))
            .expect("seeded data sources should exist");

        assert_eq!(instrument_symbol, "BTC-USD-SPOT");
        assert_eq!(source_count, 3);
    }
}
