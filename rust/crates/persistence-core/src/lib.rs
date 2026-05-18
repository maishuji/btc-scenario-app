use market_data_core::candle::Candle;
use market_data_core::snapshot::LivePriceSnapshot;
use scenario_core::{FeatureSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

pub mod models {
    use super::{Candle, FeatureSnapshot, LivePriceSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

    #[derive(Debug, Clone, PartialEq)]
    pub struct CandleRecord {
        pub candle: Candle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FeatureSnapshotRecord {
        pub snapshot: FeatureSnapshot,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LivePriceSnapshotRecord {
        pub snapshot: LivePriceSnapshot,
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
    use super::{Candle, FeatureSnapshot, LivePriceSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

    pub trait CandleRepository {
        type Error;

        fn save(&self, candle: &Candle) -> Result<(), Self::Error>;
    }

    pub trait FeatureSnapshotRepository {
        type Error;

        fn save(&self, snapshot: &FeatureSnapshot) -> Result<(), Self::Error>;
    }

    pub trait LivePriceSnapshotRepository {
        type Error;

        fn save(&self, snapshot: &LivePriceSnapshot) -> Result<(), Self::Error>;
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

pub mod sqlite {
    use std::cell::RefCell;
    use std::path::Path;

    use rusqlite::{params, Connection};

    use super::repositories::{
        CandleRepository,
        FeatureSnapshotRepository,
        LivePriceSnapshotRepository,
        RegimeSnapshotRepository,
        ScenarioSnapshotRepository,
    };
    use super::{Candle, FeatureSnapshot, LivePriceSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};
    use market_data_core::timeframe::Timeframe;
    use market_data_core::value_objects::{Price, Timestamp, Volume};
    use scenario_core::{ExpectedDirection, MarketRegimeLabel};

    const INITIAL_MIGRATION: &str = include_str!("../../../../db/migrations/0001_initial_schema.sql");

    #[derive(Debug)]
    pub struct SqliteMarketDataStore {
        connection: RefCell<Connection>,
    }

    impl SqliteMarketDataStore {
        pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
            let connection = Connection::open(path)?;
            Ok(Self {
                connection: RefCell::new(connection),
            })
        }

        pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
            let connection = Connection::open_in_memory()?;
            Ok(Self {
                connection: RefCell::new(connection),
            })
        }

        pub fn apply_migrations(&self) -> Result<(), rusqlite::Error> {
            self.connection.borrow().execute_batch(INITIAL_MIGRATION)
        }

        pub fn count_rows(&self, table_name: &str) -> Result<i64, rusqlite::Error> {
            let query = format!("SELECT COUNT(*) FROM {table_name}");
            self.connection
                .borrow()
                .query_row(query.as_str(), [], |row| row.get(0))
        }

        pub fn load_recent_candles(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<Candle>, rusqlite::Error> {
            let connection = self.connection.borrow();
            let mut statement = connection.prepare(
                "SELECT
                    instrument_id,
                    source_id,
                    timeframe,
                    open_time_ms,
                    close_time_ms,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    trade_count,
                    is_final
                 FROM candles
                 WHERE instrument_id = ?1 AND timeframe = ?2
                 ORDER BY open_time_ms DESC
                 LIMIT ?3",
            )?;
            let rows = statement.query_map(
                params![instrument_id, timeframe.as_str(), limit as i64],
                |row| {
                    let timeframe_text: String = row.get(2)?;
                    let timeframe = timeframe_text.parse::<Timeframe>().map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                        )
                    })?;

                    Ok(Candle {
                        instrument_id: row.get(0)?,
                        source_id: row.get(1)?,
                        timeframe,
                        open_time: Timestamp::new(row.get(3)?).map_err(sqlite_mapping_error)?,
                        close_time: Timestamp::new(row.get(4)?).map_err(sqlite_mapping_error)?,
                        open: Price::new(row.get(5)?).map_err(sqlite_mapping_error)?,
                        high: Price::new(row.get(6)?).map_err(sqlite_mapping_error)?,
                        low: Price::new(row.get(7)?).map_err(sqlite_mapping_error)?,
                        close: Price::new(row.get(8)?).map_err(sqlite_mapping_error)?,
                        volume: Volume::new(row.get(9)?).map_err(sqlite_mapping_error)?,
                        trade_count: row.get::<_, i64>(10)? as u64,
                        is_final: row.get::<_, i64>(11)? == 1,
                    })
                },
            )?;

            let mut candles = rows.collect::<Result<Vec<_>, _>>()?;
            candles.reverse();
            Ok(candles)
        }
    }

    fn sqlite_mapping_error(message: &'static str) -> rusqlite::Error {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Real,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, message)),
        )
    }

    fn regime_label_as_str(value: MarketRegimeLabel) -> &'static str {
        match value {
            MarketRegimeLabel::Uptrend => "uptrend",
            MarketRegimeLabel::Downtrend => "downtrend",
            MarketRegimeLabel::Range => "range",
            MarketRegimeLabel::HighVolatilityTransition => "high_volatility_transition",
        }
    }

    fn expected_direction_as_str(value: ExpectedDirection) -> &'static str {
        match value {
            ExpectedDirection::Bullish => "bullish",
            ExpectedDirection::Neutral => "neutral",
            ExpectedDirection::Bearish => "bearish",
        }
    }

    impl CandleRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn save(&self, candle: &Candle) -> Result<(), Self::Error> {
            let identifier = format!(
                "{}:{}:{}:{}",
                candle.instrument_id,
                candle.source_id,
                candle.timeframe,
                candle.open_time.0
            );
            self.connection.borrow().execute(
                "INSERT OR REPLACE INTO candles (
                    id,
                    instrument_id,
                    source_id,
                    timeframe,
                    open_time_ms,
                    close_time_ms,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    trade_count,
                    is_final,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    identifier,
                    candle.instrument_id,
                    candle.source_id,
                    candle.timeframe.as_str(),
                    candle.open_time.0,
                    candle.close_time.0,
                    candle.open.0,
                    candle.high.0,
                    candle.low.0,
                    candle.close.0,
                    candle.volume.0,
                    candle.trade_count as i64,
                    if candle.is_final { 1 } else { 0 },
                    candle.close_time.0,
                ],
            )?;

            Ok(())
        }
    }

    impl LivePriceSnapshotRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn save(&self, snapshot: &LivePriceSnapshot) -> Result<(), Self::Error> {
            let identifier = format!(
                "{}:{}:{}",
                snapshot.instrument_id, snapshot.source_id, snapshot.observed_at.0
            );
            self.connection.borrow().execute(
                "INSERT OR REPLACE INTO live_price_snapshots (
                    id,
                    instrument_id,
                    source_id,
                    last_price,
                    price_change_24h,
                    volume_24h,
                    observed_at_ms,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    identifier,
                    snapshot.instrument_id,
                    snapshot.source_id,
                    snapshot.last_price.0,
                    snapshot.price_change_24h,
                    snapshot.volume_24h.0,
                    snapshot.observed_at.0,
                    snapshot.observed_at.0,
                ],
            )?;

            Ok(())
        }
    }

    impl FeatureSnapshotRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn save(&self, snapshot: &FeatureSnapshot) -> Result<(), Self::Error> {
            let identifier = format!(
                "{}:{}:{}:{}",
                snapshot.instrument_id,
                snapshot.timeframe,
                snapshot.observed_at.0,
                "v1"
            );
            self.connection.borrow().execute(
                "INSERT OR REPLACE INTO feature_snapshots (
                    id,
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    trend_score,
                    momentum_score,
                    volatility_score,
                    volume_confirmation_score,
                    support_distance,
                    resistance_distance,
                    level_reaction_score,
                    feature_version,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    identifier,
                    snapshot.instrument_id,
                    snapshot.timeframe.as_str(),
                    snapshot.observed_at.0,
                    snapshot.trend_score,
                    snapshot.momentum_score,
                    snapshot.volatility_score,
                    snapshot.volume_confirmation_score,
                    snapshot.support_distance,
                    snapshot.resistance_distance,
                    snapshot.level_reaction_score,
                    "v1",
                    snapshot.observed_at.0,
                ],
            )?;

            Ok(())
        }
    }

    impl RegimeSnapshotRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn save(&self, snapshot: &MarketRegimeSnapshot) -> Result<(), Self::Error> {
            let identifier = format!(
                "{}:{}:{}:{}",
                snapshot.instrument_id,
                snapshot.timeframe,
                snapshot.observed_at.0,
                "v1"
            );
            self.connection.borrow().execute(
                "INSERT OR REPLACE INTO regime_snapshots (
                    id,
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    regime_label,
                    regime_score,
                    regime_version,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    identifier,
                    snapshot.instrument_id,
                    snapshot.timeframe.as_str(),
                    snapshot.observed_at.0,
                    regime_label_as_str(snapshot.regime_label),
                    snapshot.regime_score,
                    "v1",
                    snapshot.observed_at.0,
                ],
            )?;

            Ok(())
        }
    }

    impl ScenarioSnapshotRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn save(&self, snapshot: &ScenarioSnapshot) -> Result<(), Self::Error> {
            let identifier = format!(
                "{}:{}:{}:{}",
                snapshot.instrument_id,
                snapshot.timeframe,
                snapshot.observed_at.0,
                "v1"
            );
            self.connection.borrow().execute(
                "INSERT OR REPLACE INTO scenario_snapshots (
                    id,
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    bull_probability,
                    base_probability,
                    bear_probability,
                    trigger_level,
                    invalidation_level,
                    expected_direction,
                    explanation,
                    scenario_version,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    identifier,
                    snapshot.instrument_id,
                    snapshot.timeframe.as_str(),
                    snapshot.observed_at.0,
                    snapshot.bull_probability,
                    snapshot.base_probability,
                    snapshot.bear_probability,
                    snapshot.trigger_level,
                    snapshot.invalidation_level,
                    expected_direction_as_str(snapshot.expected_direction),
                    snapshot.explanation,
                    "v1",
                    snapshot.observed_at.0,
                ],
            )?;

            Ok(())
        }
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
    use market_data_core::snapshot::LivePriceSnapshot;
    use market_data_core::timeframe::Timeframe;
    use market_data_core::value_objects::{Price, Timestamp, Volume};
    use rusqlite::Connection;
    use scenario_core::{
        FeatureSnapshot,
        MarketRegimeLabel,
        MarketRegimeSnapshot,
        ScenarioSnapshot,
        ExpectedDirection,
    };

    use super::repositories::{
        CandleRepository,
        FeatureSnapshotRepository,
        LivePriceSnapshotRepository,
        RegimeSnapshotRepository,
        ScenarioSnapshotRepository,
    };
    use super::sqlite::SqliteMarketDataStore;
    use super::Candle;

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

    #[test]
    fn sqlite_store_saves_candle_rows() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store
            .apply_migrations()
            .expect("migrations should apply");

        let candle = Candle {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            source_id: "binance".to_owned(),
            timeframe: Timeframe::OneMinute,
            open_time: Timestamp::new(1_710_000_000_000).unwrap(),
            close_time: Timestamp::new(1_710_000_059_999).unwrap(),
            open: Price::new(68_000.0).unwrap(),
            high: Price::new(68_500.0).unwrap(),
            low: Price::new(67_950.0).unwrap(),
            close: Price::new(68_450.12).unwrap(),
            volume: Volume::new(123.45).unwrap(),
            trade_count: 42,
            is_final: true,
        };

        CandleRepository::save(&store, &candle).expect("candle should save");

        assert_eq!(store.count_rows("candles").unwrap(), 1);
    }

    #[test]
    fn sqlite_store_saves_live_price_snapshots() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store
            .apply_migrations()
            .expect("migrations should apply");

        let snapshot = LivePriceSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            source_id: "binance".to_owned(),
            last_price: Price::new(68_450.12).unwrap(),
            price_change_24h: 3.12,
            volume_24h: Volume::new(8_913.3).unwrap(),
            observed_at: Timestamp::new(1_710_000_000_000).unwrap(),
        };

        LivePriceSnapshotRepository::save(&store, &snapshot)
            .expect("live price snapshot should save");

        assert_eq!(store.count_rows("live_price_snapshots").unwrap(), 1);
    }

    #[test]
    fn sqlite_store_saves_analytics_snapshots() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let feature_snapshot = FeatureSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            timeframe: Timeframe::OneMinute,
            observed_at: Timestamp::new(1_710_000_120_000).unwrap(),
            trend_score: 500.0,
            momentum_score: 49.88,
            volatility_score: 200.0,
            volume_confirmation_score: 10.0,
            support_distance: 50.0,
            resistance_distance: 100.0,
            level_reaction_score: 150.0,
        };
        let regime_snapshot = MarketRegimeSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            timeframe: Timeframe::OneMinute,
            observed_at: Timestamp::new(1_710_000_120_000).unwrap(),
            regime_label: MarketRegimeLabel::Uptrend,
            regime_score: 700.0,
        };
        let scenario_snapshot = ScenarioSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            timeframe: Timeframe::OneMinute,
            observed_at: Timestamp::new(1_710_000_120_000).unwrap(),
            bull_probability: 0.55,
            base_probability: 0.30,
            bear_probability: 0.15,
            trigger_level: 100.0,
            invalidation_level: 50.0,
            expected_direction: ExpectedDirection::Bullish,
            explanation: "BTC is in an uptrend.".to_owned(),
        };

        FeatureSnapshotRepository::save(&store, &feature_snapshot).expect("feature snapshot should save");
        RegimeSnapshotRepository::save(&store, &regime_snapshot).expect("regime snapshot should save");
        ScenarioSnapshotRepository::save(&store, &scenario_snapshot).expect("scenario snapshot should save");

        assert_eq!(store.count_rows("feature_snapshots").unwrap(), 1);
        assert_eq!(store.count_rows("regime_snapshots").unwrap(), 1);
        assert_eq!(store.count_rows("scenario_snapshots").unwrap(), 1);
    }

    #[test]
    fn sqlite_store_loads_recent_candles_in_ascending_time_order() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        for (open_time, close_time, close) in [
            (1_710_000_000_000_i64, 1_710_000_059_999_i64, 68_450.12_f64),
            (1_710_000_060_000_i64, 1_710_000_119_999_i64, 68_500.00_f64),
        ] {
            let candle = Candle {
                instrument_id: "BTC-USD-SPOT".to_owned(),
                source_id: "binance".to_owned(),
                timeframe: Timeframe::OneMinute,
                open_time: Timestamp::new(open_time).unwrap(),
                close_time: Timestamp::new(close_time).unwrap(),
                open: Price::new(68_000.0).unwrap(),
                high: Price::new(68_600.0).unwrap(),
                low: Price::new(67_950.0).unwrap(),
                close: Price::new(close).unwrap(),
                volume: Volume::new(123.45).unwrap(),
                trade_count: 42,
                is_final: true,
            };

            CandleRepository::save(&store, &candle).expect("candle should save");
        }

        let candles = store
            .load_recent_candles("BTC-USD-SPOT", Timeframe::OneMinute, 2)
            .expect("recent candles should load");

        assert_eq!(candles.len(), 2);
        assert!(candles[0].open_time.0 < candles[1].open_time.0);
        assert_eq!(candles[1].close.0, 68_500.0);
    }
}
