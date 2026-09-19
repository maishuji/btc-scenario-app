use market_data_core::candle::Candle;
use market_data_core::snapshot::LivePriceSnapshot;
use scenario_core::{FeatureSnapshot, MarketRegimeSnapshot, ScenarioSnapshot};

pub fn scenario_snapshot_id(snapshot: &ScenarioSnapshot) -> String {
    format!(
        "{}:{}:{}:{}",
        snapshot.instrument_id,
        snapshot.timeframe,
        snapshot.observed_at.0,
        "v1"
    )
}

pub mod models {
    use market_data_core::timeframe::Timeframe;

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

    #[derive(Debug, Clone, PartialEq)]
    pub struct LatestMarketOverviewRecord {
        pub live_price_snapshot: LivePriceSnapshot,
        pub feature_snapshot: FeatureSnapshot,
        pub regime_snapshot: MarketRegimeSnapshot,
        pub scenario_snapshot: ScenarioSnapshot,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AlertRecord {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub alert_type: String,
        pub severity: String,
        pub message: String,
        pub triggered_at_ms: i64,
        pub scenario_snapshot_id: Option<String>,
        pub is_acknowledged: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SourceHealthRecord {
        pub source_id: String,
        pub status: String,
        pub message: String,
        pub observed_at_ms: i64,
        pub created_at_ms: i64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SourceHealthSnapshot {
        pub latest_event: SourceHealthRecord,
        pub last_successful_update_ms: Option<i64>,
    }
}

pub mod repositories {
    use market_data_core::timeframe::Timeframe;

    use super::models::{AlertRecord, CandleRecord, LatestMarketOverviewRecord, ScenarioSnapshotRecord};
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

    pub trait AlertRepository {
        type Error;

        fn save(&self, alert: &AlertRecord) -> Result<(), Self::Error>;
    }

    pub trait SourceHealthRepository {
        type Error;

        fn save(&self, event: &super::models::SourceHealthRecord) -> Result<(), Self::Error>;
    }

    pub trait MarketOverviewQueryRepository {
        type Error;

        fn load_latest_market_overview(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
        ) -> Result<Option<LatestMarketOverviewRecord>, Self::Error>;
    }

    pub trait CandleHistoryQueryRepository {
        type Error;

        fn load_candle_history(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<CandleRecord>, Self::Error>;
    }

    pub trait ScenarioHistoryQueryRepository {
        type Error;

        fn load_scenario_history(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<ScenarioSnapshotRecord>, Self::Error>;
    }

    pub trait AlertHistoryQueryRepository {
        type Error;

        fn load_alert_history(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<AlertRecord>, Self::Error>;
    }

    pub trait SourceHealthQueryRepository {
        type Error;

        fn load_source_health(&self) -> Result<Vec<super::models::SourceHealthSnapshot>, Self::Error>;
    }
}

pub mod sqlite {
    use std::cell::RefCell;
    use std::path::Path;

    use rusqlite::{params, Connection, OptionalExtension};

    use super::models::{
        AlertRecord,
        CandleRecord,
        LatestMarketOverviewRecord,
        ScenarioSnapshotRecord,
        SourceHealthRecord,
        SourceHealthSnapshot,
    };
    use super::repositories::{
        AlertHistoryQueryRepository,
        AlertRepository,
        CandleHistoryQueryRepository,
        CandleRepository,
        FeatureSnapshotRepository,
        LivePriceSnapshotRepository,
        MarketOverviewQueryRepository,
        RegimeSnapshotRepository,
        ScenarioHistoryQueryRepository,
        ScenarioSnapshotRepository,
        SourceHealthQueryRepository,
        SourceHealthRepository,
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
            self.connection.borrow().execute_batch(INITIAL_MIGRATION)?;

            if !self.has_feature_snapshot_column("support_level")? {
                self.connection.borrow().execute_batch(
                    "ALTER TABLE feature_snapshots ADD COLUMN support_level REAL NOT NULL DEFAULT 0.0",
                )?;
            }
            if !self.has_feature_snapshot_column("resistance_level")? {
                self.connection.borrow().execute_batch(
                    "ALTER TABLE feature_snapshots ADD COLUMN resistance_level REAL NOT NULL DEFAULT 0.0",
                )?;
            }

            Ok(())
        }

        fn has_feature_snapshot_column(&self, column_name: &str) -> Result<bool, rusqlite::Error> {
            let connection = self.connection.borrow();
            let mut statement = connection.prepare("PRAGMA table_info(feature_snapshots)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;

            for column in columns {
                if column? == column_name {
                    return Ok(true);
                }
            }

            Ok(false)
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

        pub fn load_recent_alerts(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<AlertRecord>, rusqlite::Error> {
            let connection = self.connection.borrow();
            let mut statement = connection.prepare(
                "SELECT
                    instrument_id,
                    timeframe,
                    alert_type,
                    severity,
                    message,
                    triggered_at_ms,
                    scenario_snapshot_id,
                    is_acknowledged
                 FROM alerts
                 WHERE instrument_id = ?1 AND timeframe = ?2
                 ORDER BY triggered_at_ms DESC
                 LIMIT ?3",
            )?;
            let rows = statement.query_map(
                params![instrument_id, timeframe.as_str(), limit as i64],
                |row| {
                    let timeframe_text: String = row.get(1)?;
                    let timeframe = timeframe_text.parse::<Timeframe>().map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                        )
                    })?;

                    Ok(AlertRecord {
                        instrument_id: row.get(0)?,
                        timeframe,
                        alert_type: row.get(2)?,
                        severity: row.get(3)?,
                        message: row.get(4)?,
                        triggered_at_ms: row.get(5)?,
                        scenario_snapshot_id: row.get(6)?,
                        is_acknowledged: row.get::<_, i64>(7)? == 1,
                    })
                },
            )?;

            let mut alerts = rows.collect::<Result<Vec<_>, _>>()?;
            alerts.reverse();
            Ok(alerts)
        }

        pub fn load_source_health_snapshots(&self) -> Result<Vec<SourceHealthSnapshot>, rusqlite::Error> {
            let connection = self.connection.borrow();
            let mut statement = connection.prepare(
                "SELECT
                    latest.source_id,
                    latest.status,
                    latest.message,
                    latest.observed_at_ms,
                    latest.created_at_ms,
                    (
                        SELECT MAX(success.observed_at_ms)
                        FROM source_health_events AS success
                        WHERE success.source_id = latest.source_id
                          AND success.status = 'healthy'
                    )
                 FROM source_health_events AS latest
                 WHERE NOT EXISTS (
                    SELECT 1
                    FROM source_health_events AS newer
                    WHERE newer.source_id = latest.source_id
                      AND (
                        newer.observed_at_ms > latest.observed_at_ms
                        OR (
                            newer.observed_at_ms = latest.observed_at_ms
                            AND newer.created_at_ms > latest.created_at_ms
                        )
                        OR (
                            newer.observed_at_ms = latest.observed_at_ms
                            AND newer.created_at_ms = latest.created_at_ms
                            AND newer.id > latest.id
                        )
                      )
                 )
                 ORDER BY latest.source_id",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(SourceHealthSnapshot {
                    latest_event: SourceHealthRecord {
                        source_id: row.get(0)?,
                        status: row.get(1)?,
                        message: row.get(2)?,
                        observed_at_ms: row.get(3)?,
                        created_at_ms: row.get(4)?,
                    },
                    last_successful_update_ms: row.get(5)?,
                })
            })?;

            rows.collect()
        }

        fn load_latest_live_price_snapshot(
            &self,
            instrument_id: &str,
        ) -> Result<Option<LivePriceSnapshot>, rusqlite::Error> {
            self.connection.borrow().query_row(
                "SELECT
                    instrument_id,
                    source_id,
                    last_price,
                    price_change_24h,
                    volume_24h,
                    observed_at_ms
                 FROM live_price_snapshots
                 WHERE instrument_id = ?1
                 ORDER BY observed_at_ms DESC
                 LIMIT 1",
                params![instrument_id],
                |row| {
                    Ok(LivePriceSnapshot {
                        instrument_id: row.get(0)?,
                        source_id: row.get(1)?,
                        last_price: Price::new(row.get(2)?).map_err(sqlite_mapping_error)?,
                        price_change_24h: row.get(3)?,
                        volume_24h: Volume::new(row.get(4)?).map_err(sqlite_mapping_error)?,
                        observed_at: Timestamp::new(row.get(5)?).map_err(sqlite_mapping_error)?,
                    })
                },
            )
            .optional()
        }

        fn load_feature_snapshot_at(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            observed_at_ms: i64,
        ) -> Result<Option<FeatureSnapshot>, rusqlite::Error> {
            self.connection.borrow().query_row(
                "SELECT
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    trend_score,
                    momentum_score,
                    volatility_score,
                    volume_confirmation_score,
                    support_level,
                    resistance_level,
                    support_distance,
                    resistance_distance,
                    level_reaction_score
                 FROM feature_snapshots
                 WHERE instrument_id = ?1 AND timeframe = ?2 AND observed_at_ms = ?3
                 LIMIT 1",
                params![instrument_id, timeframe.as_str(), observed_at_ms],
                |row| {
                    let timeframe_text: String = row.get(1)?;
                    let timeframe = timeframe_text.parse::<Timeframe>().map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                        )
                    })?;

                    Ok(FeatureSnapshot {
                        instrument_id: row.get(0)?,
                        timeframe,
                        observed_at: Timestamp::new(row.get(2)?).map_err(sqlite_mapping_error)?,
                        trend_score: row.get(3)?,
                        momentum_score: row.get(4)?,
                        volatility_score: row.get(5)?,
                        volume_confirmation_score: row.get(6)?,
                        support_level: row.get(7)?,
                        resistance_level: row.get(8)?,
                        support_distance: row.get(9)?,
                        resistance_distance: row.get(10)?,
                        level_reaction_score: row.get(11)?,
                    })
                },
            )
            .optional()
        }

        fn load_regime_snapshot_at(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            observed_at_ms: i64,
        ) -> Result<Option<MarketRegimeSnapshot>, rusqlite::Error> {
            self.connection.borrow().query_row(
                "SELECT
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    regime_label,
                    regime_score
                 FROM regime_snapshots
                 WHERE instrument_id = ?1 AND timeframe = ?2 AND observed_at_ms = ?3
                 LIMIT 1",
                params![instrument_id, timeframe.as_str(), observed_at_ms],
                |row| {
                    let timeframe_text: String = row.get(1)?;
                    let timeframe = timeframe_text.parse::<Timeframe>().map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                        )
                    })?;
                    let regime_label = parse_regime_label(row.get::<_, String>(3)?.as_str())?;

                    Ok(MarketRegimeSnapshot {
                        instrument_id: row.get(0)?,
                        timeframe,
                        observed_at: Timestamp::new(row.get(2)?).map_err(sqlite_mapping_error)?,
                        regime_label,
                        regime_score: row.get(4)?,
                    })
                },
            )
            .optional()
        }

        pub fn load_latest_regime_snapshot(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
        ) -> Result<Option<MarketRegimeSnapshot>, rusqlite::Error> {
            self.connection.borrow().query_row(
                "SELECT
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    regime_label,
                    regime_score
                 FROM regime_snapshots
                 WHERE instrument_id = ?1 AND timeframe = ?2
                 ORDER BY observed_at_ms DESC
                 LIMIT 1",
                params![instrument_id, timeframe.as_str()],
                |row| {
                    let timeframe_text: String = row.get(1)?;
                    let timeframe = timeframe_text.parse::<Timeframe>().map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                        )
                    })?;
                    let regime_label = parse_regime_label(row.get::<_, String>(3)?.as_str())?;

                    Ok(MarketRegimeSnapshot {
                        instrument_id: row.get(0)?,
                        timeframe,
                        observed_at: Timestamp::new(row.get(2)?).map_err(sqlite_mapping_error)?,
                        regime_label,
                        regime_score: row.get(4)?,
                    })
                },
            )
            .optional()
        }

        pub fn load_latest_scenario_snapshot(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
        ) -> Result<Option<ScenarioSnapshot>, rusqlite::Error> {
            self.connection.borrow().query_row(
                "SELECT
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    bull_probability,
                    base_probability,
                    bear_probability,
                    trigger_level,
                    invalidation_level,
                    expected_direction,
                    explanation
                 FROM scenario_snapshots
                 WHERE instrument_id = ?1 AND timeframe = ?2
                 ORDER BY observed_at_ms DESC
                 LIMIT 1",
                params![instrument_id, timeframe.as_str()],
                |row| {
                    let timeframe_text: String = row.get(1)?;
                    let timeframe = timeframe_text.parse::<Timeframe>().map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                        )
                    })?;
                    let expected_direction = parse_expected_direction(row.get::<_, String>(8)?.as_str())?;

                    Ok(ScenarioSnapshot {
                        instrument_id: row.get(0)?,
                        timeframe,
                        observed_at: Timestamp::new(row.get(2)?).map_err(sqlite_mapping_error)?,
                        bull_probability: row.get(3)?,
                        base_probability: row.get(4)?,
                        bear_probability: row.get(5)?,
                        trigger_level: row.get(6)?,
                        invalidation_level: row.get(7)?,
                        expected_direction,
                        explanation: row.get(9)?,
                    })
                },
            )
            .optional()
        }

        fn load_recent_scenario_snapshots(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<ScenarioSnapshot>, rusqlite::Error> {
            let connection = self.connection.borrow();
            let mut statement = connection.prepare(
                "SELECT
                    instrument_id,
                    timeframe,
                    observed_at_ms,
                    bull_probability,
                    base_probability,
                    bear_probability,
                    trigger_level,
                    invalidation_level,
                    expected_direction,
                    explanation
                 FROM scenario_snapshots
                 WHERE instrument_id = ?1 AND timeframe = ?2
                 ORDER BY observed_at_ms DESC
                 LIMIT ?3",
            )?;
            let rows = statement.query_map(
                params![instrument_id, timeframe.as_str(), limit as i64],
                |row| {
                    let timeframe_text: String = row.get(1)?;
                    let timeframe = timeframe_text.parse::<Timeframe>().map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                        )
                    })?;
                    let expected_direction = parse_expected_direction(row.get::<_, String>(8)?.as_str())?;

                    Ok(ScenarioSnapshot {
                        instrument_id: row.get(0)?,
                        timeframe,
                        observed_at: Timestamp::new(row.get(2)?).map_err(sqlite_mapping_error)?,
                        bull_probability: row.get(3)?,
                        base_probability: row.get(4)?,
                        bear_probability: row.get(5)?,
                        trigger_level: row.get(6)?,
                        invalidation_level: row.get(7)?,
                        expected_direction,
                        explanation: row.get(9)?,
                    })
                },
            )?;

            let mut scenarios = rows.collect::<Result<Vec<_>, _>>()?;
            scenarios.reverse();
            Ok(scenarios)
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

    fn parse_regime_label(value: &str) -> Result<MarketRegimeLabel, rusqlite::Error> {
        match value {
            "uptrend" => Ok(MarketRegimeLabel::Uptrend),
            "downtrend" => Ok(MarketRegimeLabel::Downtrend),
            "range" => Ok(MarketRegimeLabel::Range),
            "high_volatility_transition" => Ok(MarketRegimeLabel::HighVolatilityTransition),
            _ => Err(sqlite_text_mapping_error(value)),
        }
    }

    fn expected_direction_as_str(value: ExpectedDirection) -> &'static str {
        match value {
            ExpectedDirection::Bullish => "bullish",
            ExpectedDirection::Neutral => "neutral",
            ExpectedDirection::Bearish => "bearish",
        }
    }

    fn parse_expected_direction(value: &str) -> Result<ExpectedDirection, rusqlite::Error> {
        match value {
            "bullish" => Ok(ExpectedDirection::Bullish),
            "neutral" => Ok(ExpectedDirection::Neutral),
            "bearish" => Ok(ExpectedDirection::Bearish),
            _ => Err(sqlite_text_mapping_error(value)),
        }
    }

    fn sqlite_text_mapping_error(value: &str) -> rusqlite::Error {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsupported SQLite text enum value: {value}"),
            )),
        )
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
                    support_level,
                    resistance_level,
                    support_distance,
                    resistance_distance,
                    level_reaction_score,
                    feature_version,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    identifier,
                    snapshot.instrument_id,
                    snapshot.timeframe.as_str(),
                    snapshot.observed_at.0,
                    snapshot.trend_score,
                    snapshot.momentum_score,
                    snapshot.volatility_score,
                    snapshot.volume_confirmation_score,
                    snapshot.support_level,
                    snapshot.resistance_level,
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
            let identifier = super::scenario_snapshot_id(snapshot);
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

    impl AlertRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn save(&self, alert: &AlertRecord) -> Result<(), Self::Error> {
            let identifier = format!(
                "{}:{}:{}:{}",
                alert.instrument_id,
                alert.timeframe,
                alert.alert_type,
                alert.triggered_at_ms,
            );
            self.connection.borrow().execute(
                "INSERT OR REPLACE INTO alerts (
                    id,
                    instrument_id,
                    timeframe,
                    alert_type,
                    severity,
                    message,
                    triggered_at_ms,
                    scenario_snapshot_id,
                    is_acknowledged,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    identifier,
                    alert.instrument_id,
                    alert.timeframe.as_str(),
                    alert.alert_type,
                    alert.severity,
                    alert.message,
                    alert.triggered_at_ms,
                    alert.scenario_snapshot_id,
                    if alert.is_acknowledged { 1_i64 } else { 0_i64 },
                    alert.triggered_at_ms,
                ],
            )?;

            Ok(())
        }
    }

    impl SourceHealthRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn save(&self, event: &SourceHealthRecord) -> Result<(), Self::Error> {
            let identifier = format!(
                "{}:{}:{}",
                event.source_id, event.status, event.observed_at_ms
            );
            self.connection.borrow().execute(
                "INSERT OR REPLACE INTO source_health_events (
                    id,
                    source_id,
                    status,
                    message,
                    observed_at_ms,
                    created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    identifier,
                    event.source_id,
                    event.status,
                    event.message,
                    event.observed_at_ms,
                    event.created_at_ms,
                ],
            )?;

            Ok(())
        }
    }

    impl MarketOverviewQueryRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn load_latest_market_overview(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
        ) -> Result<Option<LatestMarketOverviewRecord>, Self::Error> {
            let Some(scenario_snapshot) = self.load_latest_scenario_snapshot(instrument_id, timeframe)? else {
                return Ok(None);
            };
            let Some(feature_snapshot) = self.load_feature_snapshot_at(
                instrument_id,
                timeframe,
                scenario_snapshot.observed_at.0,
            )? else {
                return Ok(None);
            };
            let Some(regime_snapshot) = self.load_regime_snapshot_at(
                instrument_id,
                timeframe,
                scenario_snapshot.observed_at.0,
            )? else {
                return Ok(None);
            };
            let Some(live_price_snapshot) = self.load_latest_live_price_snapshot(instrument_id)? else {
                return Ok(None);
            };

            Ok(Some(LatestMarketOverviewRecord {
                live_price_snapshot,
                feature_snapshot,
                regime_snapshot,
                scenario_snapshot,
            }))
        }
    }

    impl CandleHistoryQueryRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn load_candle_history(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<CandleRecord>, Self::Error> {
            self.load_recent_candles(instrument_id, timeframe, limit)
                .map(|candles| candles.into_iter().map(|candle| CandleRecord { candle }).collect())
        }
    }

    impl ScenarioHistoryQueryRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn load_scenario_history(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<ScenarioSnapshotRecord>, Self::Error> {
            self.load_recent_scenario_snapshots(instrument_id, timeframe, limit)
                .map(|snapshots| snapshots.into_iter().map(|snapshot| ScenarioSnapshotRecord { snapshot }).collect())
        }
    }

    impl AlertHistoryQueryRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn load_alert_history(
            &self,
            instrument_id: &str,
            timeframe: Timeframe,
            limit: usize,
        ) -> Result<Vec<AlertRecord>, Self::Error> {
            self.load_recent_alerts(instrument_id, timeframe, limit)
        }
    }

    impl SourceHealthQueryRepository for SqliteMarketDataStore {
        type Error = rusqlite::Error;

        fn load_source_health(&self) -> Result<Vec<SourceHealthSnapshot>, Self::Error> {
            self.load_source_health_snapshots()
        }
    }
}

pub mod queries {
    use market_data_core::timeframe::Timeframe;

    use super::models::{
        AlertRecord,
        CandleRecord,
        LatestMarketOverviewRecord,
        ScenarioSnapshotRecord,
        SourceHealthSnapshot,
    };
    use super::repositories::{
        AlertHistoryQueryRepository,
        CandleHistoryQueryRepository,
        MarketOverviewQueryRepository,
        ScenarioHistoryQueryRepository,
        SourceHealthQueryRepository,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LatestMarketOverviewQuery {
        pub instrument_id: String,
        pub timeframe: Timeframe,
    }

    impl LatestMarketOverviewQuery {
        pub fn for_instrument(instrument_id: impl Into<String>, timeframe: Timeframe) -> Self {
            Self {
                instrument_id: instrument_id.into(),
                timeframe,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct MarketOverviewQueryService;

    impl MarketOverviewQueryService {
        pub fn load_latest<R>(
            self,
            repository: &R,
            query: &LatestMarketOverviewQuery,
        ) -> Result<Option<LatestMarketOverviewRecord>, R::Error>
        where
            R: MarketOverviewQueryRepository,
        {
            repository.load_latest_market_overview(&query.instrument_id, query.timeframe)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CandleHistoryQuery {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub limit: usize,
    }

    impl CandleHistoryQuery {
        pub fn for_instrument(instrument_id: impl Into<String>, timeframe: Timeframe, limit: usize) -> Self {
            Self {
                instrument_id: instrument_id.into(),
                timeframe,
                limit,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct CandleHistoryQueryService;

    impl CandleHistoryQueryService {
        pub fn load<R>(self, repository: &R, query: &CandleHistoryQuery) -> Result<Vec<CandleRecord>, R::Error>
        where
            R: CandleHistoryQueryRepository,
        {
            repository.load_candle_history(&query.instrument_id, query.timeframe, query.limit)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ScenarioHistoryQuery {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub limit: usize,
    }

    impl ScenarioHistoryQuery {
        pub fn for_instrument(instrument_id: impl Into<String>, timeframe: Timeframe, limit: usize) -> Self {
            Self {
                instrument_id: instrument_id.into(),
                timeframe,
                limit,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct ScenarioHistoryQueryService;

    impl ScenarioHistoryQueryService {
        pub fn load<R>(
            self,
            repository: &R,
            query: &ScenarioHistoryQuery,
        ) -> Result<Vec<ScenarioSnapshotRecord>, R::Error>
        where
            R: ScenarioHistoryQueryRepository,
        {
            repository.load_scenario_history(&query.instrument_id, query.timeframe, query.limit)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AlertHistoryQuery {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub limit: usize,
    }

    impl AlertHistoryQuery {
        pub fn for_instrument(instrument_id: impl Into<String>, timeframe: Timeframe, limit: usize) -> Self {
            Self {
                instrument_id: instrument_id.into(),
                timeframe,
                limit,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct AlertHistoryQueryService;

    impl AlertHistoryQueryService {
        pub fn load<R>(self, repository: &R, query: &AlertHistoryQuery) -> Result<Vec<AlertRecord>, R::Error>
        where
            R: AlertHistoryQueryRepository,
        {
            repository.load_alert_history(&query.instrument_id, query.timeframe, query.limit)
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct SourceHealthQuery;

    #[derive(Debug, Default, Clone, Copy)]
    pub struct SourceHealthQueryService;

    impl SourceHealthQueryService {
        pub fn load<R>(
            self,
            repository: &R,
            _query: &SourceHealthQuery,
        ) -> Result<Vec<SourceHealthSnapshot>, R::Error>
        where
            R: SourceHealthQueryRepository,
        {
            repository.load_source_health()
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
    use rusqlite::params;
    use rusqlite::Connection;
    use scenario_core::{
        ExpectedDirection,
        FeatureSnapshot,
        MarketRegimeLabel,
        MarketRegimeSnapshot,
        ScenarioSnapshot,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::repositories::{
        AlertHistoryQueryRepository,
        AlertRepository,
        CandleHistoryQueryRepository,
        CandleRepository,
        FeatureSnapshotRepository,
        LivePriceSnapshotRepository,
        MarketOverviewQueryRepository,
        RegimeSnapshotRepository,
        ScenarioHistoryQueryRepository,
        ScenarioSnapshotRepository,
        SourceHealthQueryRepository,
        SourceHealthRepository,
    };
    use super::sqlite::SqliteMarketDataStore;
    use super::Candle;

    fn temp_db_path(test_name: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("{test_name}-{unique}.sqlite3"));
        path.to_string_lossy().into_owned()
    }

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
    fn upgrades_existing_feature_snapshot_schema_with_level_columns() {
        let db_path = temp_db_path("feature-level-migration");
        let connection = Connection::open(&db_path).expect("sqlite connection should open");
        connection
            .execute_batch(
                "CREATE TABLE feature_snapshots (
                    id TEXT PRIMARY KEY,
                    instrument_id TEXT NOT NULL,
                    timeframe TEXT NOT NULL,
                    observed_at_ms INTEGER NOT NULL,
                    trend_score REAL NOT NULL,
                    momentum_score REAL NOT NULL,
                    volatility_score REAL NOT NULL,
                    volume_confirmation_score REAL NOT NULL,
                    support_distance REAL NOT NULL,
                    resistance_distance REAL NOT NULL,
                    level_reaction_score REAL NOT NULL,
                    feature_version TEXT NOT NULL,
                    created_at_ms INTEGER NOT NULL
                )",
            )
            .expect("legacy feature snapshot schema should be created");
        drop(connection);

        let store = SqliteMarketDataStore::open(&db_path).expect("sqlite store should open");
        store.apply_migrations().expect("migration should upgrade schema");

        let connection = Connection::open(&db_path).expect("sqlite connection should reopen");
        let columns = connection
            .prepare("PRAGMA table_info(feature_snapshots)")
            .expect("feature snapshot columns should be queryable")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("feature snapshot column query should succeed")
            .collect::<Result<Vec<_>, _>>()
            .expect("feature snapshot column names should load");

        assert!(columns.iter().any(|column| column == "support_level"));
        assert!(columns.iter().any(|column| column == "resistance_level"));
        let _ = std::fs::remove_file(db_path);
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
            support_level: 68_400.0,
            resistance_level: 68_550.0,
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
    fn sqlite_store_saves_alert_rows() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let alert = super::models::AlertRecord {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            timeframe: Timeframe::OneMinute,
            alert_type: "scenario_shifted".to_owned(),
            severity: "warning".to_owned(),
            message: "Scenario direction shifted from bearish to bullish".to_owned(),
            triggered_at_ms: 1_710_000_120_000,
            scenario_snapshot_id: None,
            is_acknowledged: false,
        };

        AlertRepository::save(&store, &alert).expect("alert should save");

        assert_eq!(store.count_rows("alerts").unwrap(), 1);
    }

    #[test]
    fn sqlite_store_loads_latest_source_health_and_last_success() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        SourceHealthRepository::save(
            &store,
            &super::models::SourceHealthRecord {
                source_id: "binance".to_owned(),
                status: "healthy".to_owned(),
                message: "Binance sync succeeded".to_owned(),
                observed_at_ms: 1_710_000_000_000,
                created_at_ms: 1_710_000_000_000,
            },
        )
        .expect("healthy source event should save");
        SourceHealthRepository::save(
            &store,
            &super::models::SourceHealthRecord {
                source_id: "binance".to_owned(),
                status: "unavailable".to_owned(),
                message: "Binance sync failed".to_owned(),
                observed_at_ms: 1_710_000_060_000,
                created_at_ms: 1_710_000_060_000,
            },
        )
        .expect("unavailable source event should save");

        let health = SourceHealthQueryRepository::load_source_health(&store)
            .expect("source health should load");

        assert_eq!(health.len(), 1);
        assert_eq!(health[0].latest_event.status, "unavailable");
        assert_eq!(health[0].last_successful_update_ms, Some(1_710_000_000_000));
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

    #[test]
    fn sqlite_store_loads_latest_market_overview() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let live_snapshot = LivePriceSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            source_id: "binance".to_owned(),
            last_price: Price::new(68_550.0).unwrap(),
            price_change_24h: 2.5,
            volume_24h: Volume::new(8_900.0).unwrap(),
            observed_at: Timestamp::new(1_710_000_180_000).unwrap(),
        };
        let feature_snapshot = FeatureSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            timeframe: Timeframe::OneMinute,
            observed_at: Timestamp::new(1_710_000_120_000).unwrap(),
            trend_score: 500.0,
            momentum_score: 50.0,
            volatility_score: 200.0,
            volume_confirmation_score: 25.0,
            support_level: 68_400.0,
            resistance_level: 68_700.0,
            support_distance: 100.0,
            resistance_distance: 150.0,
            level_reaction_score: 250.0,
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
            trigger_level: 150.0,
            invalidation_level: 100.0,
            expected_direction: ExpectedDirection::Bullish,
            explanation: "BTC is in an uptrend.".to_owned(),
        };

        LivePriceSnapshotRepository::save(&store, &live_snapshot).expect("live snapshot should save");
        FeatureSnapshotRepository::save(&store, &feature_snapshot).expect("feature snapshot should save");
        RegimeSnapshotRepository::save(&store, &regime_snapshot).expect("regime snapshot should save");
        ScenarioSnapshotRepository::save(&store, &scenario_snapshot).expect("scenario snapshot should save");

        let overview = MarketOverviewQueryRepository::load_latest_market_overview(
            &store,
            "BTC-USD-SPOT",
            Timeframe::OneMinute,
        )
        .expect("latest market overview query should succeed")
        .expect("latest market overview should exist");

        assert_eq!(overview.live_price_snapshot.last_price.0, 68_550.0);
        assert_eq!(overview.feature_snapshot.trend_score, 500.0);
        assert_eq!(overview.feature_snapshot.support_level, 68_400.0);
        assert_eq!(overview.feature_snapshot.resistance_level, 68_700.0);
        assert_eq!(overview.regime_snapshot.regime_label, MarketRegimeLabel::Uptrend);
        assert_eq!(overview.scenario_snapshot.expected_direction, ExpectedDirection::Bullish);
    }

    #[test]
    fn sqlite_store_loads_candle_history_records() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        for (open_time, close_time, close) in [
            (1_710_000_000_000_i64, 1_710_000_059_999_i64, 68_450.12_f64),
            (1_710_000_060_000_i64, 1_710_000_119_999_i64, 68_500.00_f64),
            (1_710_000_120_000_i64, 1_710_000_179_999_i64, 68_650.00_f64),
        ] {
            let candle = Candle {
                instrument_id: "BTC-USD-SPOT".to_owned(),
                source_id: "binance".to_owned(),
                timeframe: Timeframe::OneMinute,
                open_time: Timestamp::new(open_time).unwrap(),
                close_time: Timestamp::new(close_time).unwrap(),
                open: Price::new(68_000.0).unwrap(),
                high: Price::new(68_700.0).unwrap(),
                low: Price::new(67_950.0).unwrap(),
                close: Price::new(close).unwrap(),
                volume: Volume::new(123.45).unwrap(),
                trade_count: 42,
                is_final: true,
            };

            CandleRepository::save(&store, &candle).expect("candle should save");
        }

        let candles = CandleHistoryQueryRepository::load_candle_history(
            &store,
            "BTC-USD-SPOT",
            Timeframe::OneMinute,
            2,
        )
        .expect("candle history should load");

        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].candle.open_time.0, 1_710_000_060_000);
        assert_eq!(candles[1].candle.open_time.0, 1_710_000_120_000);
    }

    #[test]
    fn sqlite_store_loads_scenario_history_records() {
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        for observed_at in [1_710_000_060_000_i64, 1_710_000_120_000_i64, 1_710_000_180_000_i64] {
            let scenario_snapshot = ScenarioSnapshot {
                instrument_id: "BTC-USD-SPOT".to_owned(),
                timeframe: Timeframe::OneMinute,
                observed_at: Timestamp::new(observed_at).unwrap(),
                bull_probability: 0.55,
                base_probability: 0.30,
                bear_probability: 0.15,
                trigger_level: 150.0,
                invalidation_level: 100.0,
                expected_direction: ExpectedDirection::Bullish,
                explanation: format!("BTC scenario at {observed_at}"),
            };

            ScenarioSnapshotRepository::save(&store, &scenario_snapshot).expect("scenario snapshot should save");
        }

        let scenarios = ScenarioHistoryQueryRepository::load_scenario_history(
            &store,
            "BTC-USD-SPOT",
            Timeframe::OneMinute,
            2,
        )
        .expect("scenario history should load");

        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios[0].snapshot.observed_at.0, 1_710_000_120_000);
        assert_eq!(scenarios[1].snapshot.observed_at.0, 1_710_000_180_000);
    }

    #[test]
    fn sqlite_store_loads_alert_history_records() {
        let db_path = temp_db_path("alert-history-query");
        let store = SqliteMarketDataStore::open(&db_path).expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let connection = Connection::open(&db_path).expect("sqlite connection should open");
        for (id, triggered_at_ms, message, is_acknowledged) in [
            ("alert-1", 1_710_000_060_000_i64, "Regime changed to uptrend", 0_i64),
            ("alert-2", 1_710_000_120_000_i64, "Scenario shifted bullish", 1_i64),
            ("alert-3", 1_710_000_180_000_i64, "Trigger crossed above resistance", 0_i64),
        ] {
            connection
                .execute(
                    "INSERT INTO alerts (
                        id,
                        instrument_id,
                        timeframe,
                        alert_type,
                        severity,
                        message,
                        triggered_at_ms,
                        scenario_snapshot_id,
                        is_acknowledged,
                        created_at_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9)",
                    params![
                        id,
                        "BTC-USD-SPOT",
                        "1m",
                        "scenario_shifted",
                        "warning",
                        message,
                        triggered_at_ms,
                        is_acknowledged,
                        triggered_at_ms,
                    ],
                )
                .expect("alert row should insert");
        }

        let alerts = AlertHistoryQueryRepository::load_alert_history(
            &store,
            "BTC-USD-SPOT",
            Timeframe::OneMinute,
            2,
        )
        .expect("alert history should load");

        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0].triggered_at_ms, 1_710_000_120_000);
        assert_eq!(alerts[0].severity, "warning");
        assert!(alerts[0].is_acknowledged);
        assert_eq!(alerts[1].triggered_at_ms, 1_710_000_180_000);
        assert_eq!(alerts[1].message, "Trigger crossed above resistance");

        let _ = std::fs::remove_file(db_path);
    }
}
