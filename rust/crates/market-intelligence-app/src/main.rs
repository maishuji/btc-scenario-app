mod config {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AppConfig {
        pub instrument_symbol: String,
        pub primary_exchange_symbol: String,
        pub market_data_db_path: String,
        pub api_bind_address: String,
        pub live_sync_message_limit: usize,
        pub api_sync_interval_secs: u64,
        pub source_health_stale_after_secs: u64,
    }

    impl Default for AppConfig {
        fn default() -> Self {
            Self {
                instrument_symbol: "BTC-USD-SPOT".to_owned(),
                primary_exchange_symbol: "BTCUSDT".to_owned(),
                market_data_db_path: "var/market-data.sqlite3".to_owned(),
                api_bind_address: "127.0.0.1:3000".to_owned(),
                live_sync_message_limit: 2,
                api_sync_interval_secs: 30,
                source_health_stale_after_secs: 90,
            }
        }
    }

    impl AppConfig {
        pub fn source_health_stale_after_ms(&self) -> i64 {
            self.source_health_stale_after_secs
                .min(i64::MAX as u64 / 1_000)
                .saturating_mul(1_000) as i64
        }
    }
}

mod api {
    use axum::extract::{Query, State};
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum::{Json, Router};
    use market_data_core::timeframe::Timeframe;
    use market_data_infrastructure::health::{SourceHealthMonitor, SourceStatus};
    use persistence_core::models::{AlertRecord, CandleRecord, LatestMarketOverviewRecord, ScenarioSnapshotRecord};
    use persistence_core::queries::{
        AlertHistoryQuery,
        AlertHistoryQueryService,
        CandleHistoryQuery,
        CandleHistoryQueryService,
        LatestMarketOverviewQuery,
        MarketOverviewQueryService,
        ScenarioHistoryQuery,
        ScenarioHistoryQueryService,
        SourceHealthQuery,
        SourceHealthQueryService,
    };
    use persistence_core::sqlite::SqliteMarketDataStore;
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::config::AppConfig;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ApiState {
        pub market_data_db_path: String,
        pub instrument_id: String,
        pub source_health_stale_after_ms: i64,
    }

    impl ApiState {
        pub fn from_config(config: &AppConfig) -> Self {
            Self {
                market_data_db_path: config.market_data_db_path.clone(),
                instrument_id: config.instrument_symbol.clone(),
                source_health_stale_after_ms: config.source_health_stale_after_ms(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct MarketOverviewResponse {
        pub instrument_id: String,
        pub timeframe: String,
        pub observed_at_ms: i64,
        pub last_price: f64,
        pub price_change_24h: f64,
        pub volume_24h: f64,
        pub trend_score: f64,
        pub momentum_score: f64,
        pub volatility_score: f64,
        pub volume_confirmation_score: f64,
        pub support_level: f64,
        pub resistance_level: f64,
        pub support_distance: f64,
        pub resistance_distance: f64,
        pub level_reaction_score: f64,
        pub regime_label: String,
        pub regime_score: f64,
        pub bull_probability: f64,
        pub base_probability: f64,
        pub bear_probability: f64,
        pub trigger_level: f64,
        pub invalidation_level: f64,
        pub expected_direction: String,
        pub explanation: String,
    }

    impl MarketOverviewResponse {
        pub fn from_record(record: LatestMarketOverviewRecord) -> Self {
            Self {
                instrument_id: record.scenario_snapshot.instrument_id,
                timeframe: record.scenario_snapshot.timeframe.as_str().to_owned(),
                observed_at_ms: record.scenario_snapshot.observed_at.0,
                last_price: record.live_price_snapshot.last_price.0,
                price_change_24h: record.live_price_snapshot.price_change_24h,
                volume_24h: record.live_price_snapshot.volume_24h.0,
                trend_score: record.feature_snapshot.trend_score,
                momentum_score: record.feature_snapshot.momentum_score,
                volatility_score: record.feature_snapshot.volatility_score,
                volume_confirmation_score: record.feature_snapshot.volume_confirmation_score,
                support_level: record.feature_snapshot.support_level,
                resistance_level: record.feature_snapshot.resistance_level,
                support_distance: record.feature_snapshot.support_distance,
                resistance_distance: record.feature_snapshot.resistance_distance,
                level_reaction_score: record.feature_snapshot.level_reaction_score,
                regime_label: regime_label_as_str(record.regime_snapshot.regime_label).to_owned(),
                regime_score: record.regime_snapshot.regime_score,
                bull_probability: record.scenario_snapshot.bull_probability,
                base_probability: record.scenario_snapshot.base_probability,
                bear_probability: record.scenario_snapshot.bear_probability,
                trigger_level: record.scenario_snapshot.trigger_level,
                invalidation_level: record.scenario_snapshot.invalidation_level,
                expected_direction: expected_direction_as_str(record.scenario_snapshot.expected_direction).to_owned(),
                explanation: record.scenario_snapshot.explanation,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    pub struct ApiListQuery {
        pub timeframe: Option<String>,
        pub limit: Option<usize>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct CandleResponse {
        pub instrument_id: String,
        pub source_id: String,
        pub timeframe: String,
        pub open_time_ms: i64,
        pub close_time_ms: i64,
        pub open: f64,
        pub high: f64,
        pub low: f64,
        pub close: f64,
        pub volume: f64,
        pub trade_count: u64,
        pub is_final: bool,
    }

    impl CandleResponse {
        fn from_record(record: CandleRecord) -> Self {
            Self {
                instrument_id: record.candle.instrument_id,
                source_id: record.candle.source_id,
                timeframe: record.candle.timeframe.as_str().to_owned(),
                open_time_ms: record.candle.open_time.0,
                close_time_ms: record.candle.close_time.0,
                open: record.candle.open.0,
                high: record.candle.high.0,
                low: record.candle.low.0,
                close: record.candle.close.0,
                volume: record.candle.volume.0,
                trade_count: record.candle.trade_count,
                is_final: record.candle.is_final,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct ScenarioHistoryResponse {
        pub instrument_id: String,
        pub timeframe: String,
        pub observed_at_ms: i64,
        pub bull_probability: f64,
        pub base_probability: f64,
        pub bear_probability: f64,
        pub trigger_level: f64,
        pub invalidation_level: f64,
        pub expected_direction: String,
        pub explanation: String,
    }

    impl ScenarioHistoryResponse {
        fn from_record(record: ScenarioSnapshotRecord) -> Self {
            Self {
                instrument_id: record.snapshot.instrument_id,
                timeframe: record.snapshot.timeframe.as_str().to_owned(),
                observed_at_ms: record.snapshot.observed_at.0,
                bull_probability: record.snapshot.bull_probability,
                base_probability: record.snapshot.base_probability,
                bear_probability: record.snapshot.bear_probability,
                trigger_level: record.snapshot.trigger_level,
                invalidation_level: record.snapshot.invalidation_level,
                expected_direction: expected_direction_as_str(record.snapshot.expected_direction).to_owned(),
                explanation: record.snapshot.explanation,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct AlertResponse {
        pub instrument_id: String,
        pub timeframe: String,
        pub alert_type: String,
        pub severity: String,
        pub message: String,
        pub triggered_at_ms: i64,
        pub is_acknowledged: bool,
    }

    impl AlertResponse {
        fn from_record(record: AlertRecord) -> Self {
            Self {
                instrument_id: record.instrument_id,
                timeframe: record.timeframe.as_str().to_owned(),
                alert_type: record.alert_type,
                severity: record.severity,
                message: record.message,
                triggered_at_ms: record.triggered_at_ms,
                is_acknowledged: record.is_acknowledged,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct SourceHealthResponse {
        pub source_id: String,
        pub status: String,
        pub message: String,
        pub observed_at_ms: i64,
        pub last_successful_update_ms: Option<i64>,
        pub age_ms: Option<i64>,
    }

    pub fn build_router(state: ApiState) -> Router {
        Router::new()
            .route("/api/market-overview", get(get_market_overview))
            .route("/api/candles", get(get_candles))
            .route("/api/scenario-history", get(get_scenario_history))
            .route("/api/alerts", get(get_alerts))
            .route("/api/source-health", get(get_source_health))
            .with_state(state)
    }

    pub async fn serve(config: &AppConfig) -> Result<(), String> {
        if let Err(error) = crate::runtime::run_sync_cycle(config).await {
            eprintln!("initial market data sync failed: {error}");
        }

        crate::runtime::spawn_periodic_sync(config.clone());

        let state = ApiState::from_config(config);
        let router = build_router(state);
        let listener = tokio::net::TcpListener::bind(&config.api_bind_address)
            .await
            .map_err(|error| error.to_string())?;

        axum::serve(listener, router)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn get_market_overview(
        State(state): State<ApiState>,
        Query(params): Query<ApiListQuery>,
    ) -> Result<Json<MarketOverviewResponse>, StatusCode> {
        let store = open_store(&state)?;
        let timeframe = parse_timeframe(params.timeframe.as_deref())?;

        let query = LatestMarketOverviewQuery::for_instrument(state.instrument_id, timeframe);
        let overview = MarketOverviewQueryService
            .load_latest(&store, &query)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(MarketOverviewResponse::from_record(overview)))
    }

    pub async fn get_candles(
        State(state): State<ApiState>,
        Query(params): Query<ApiListQuery>,
    ) -> Result<Json<Vec<CandleResponse>>, StatusCode> {
        let store = open_store(&state)?;
        let timeframe = parse_timeframe(params.timeframe.as_deref())?;
        let limit = normalize_limit(params.limit, 120);
        let query = CandleHistoryQuery::for_instrument(state.instrument_id, timeframe, limit);
        let candles = CandleHistoryQueryService
            .load(&store, &query)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(candles.into_iter().map(CandleResponse::from_record).collect()))
    }

    pub async fn get_scenario_history(
        State(state): State<ApiState>,
        Query(params): Query<ApiListQuery>,
    ) -> Result<Json<Vec<ScenarioHistoryResponse>>, StatusCode> {
        let store = open_store(&state)?;
        let timeframe = parse_timeframe(params.timeframe.as_deref())?;
        let limit = normalize_limit(params.limit, 50);
        let query = ScenarioHistoryQuery::for_instrument(state.instrument_id, timeframe, limit);
        let scenarios = ScenarioHistoryQueryService
            .load(&store, &query)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(
            scenarios
                .into_iter()
                .map(ScenarioHistoryResponse::from_record)
                .collect(),
        ))
    }

    pub async fn get_alerts(
        State(state): State<ApiState>,
        Query(params): Query<ApiListQuery>,
    ) -> Result<Json<Vec<AlertResponse>>, StatusCode> {
        let store = open_store(&state)?;
        let timeframe = parse_timeframe(params.timeframe.as_deref())?;
        let limit = normalize_limit(params.limit, 50);
        let query = AlertHistoryQuery::for_instrument(state.instrument_id, timeframe, limit);
        let alerts = AlertHistoryQueryService
            .load(&store, &query)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(
            alerts
                .into_iter()
                .map(AlertResponse::from_record)
                .collect(),
        ))
    }

    pub async fn get_source_health(
        State(state): State<ApiState>,
    ) -> Result<Json<Vec<SourceHealthResponse>>, StatusCode> {
        let store = open_store(&state)?;
        let query = SourceHealthQuery;
        let health = SourceHealthQueryService
            .load(&store, &query)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let now_ms = current_time_ms().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let monitor = SourceHealthMonitor;

        Ok(Json(
            health
                .into_iter()
                .map(|snapshot| {
                    let last_successful_update_ms = snapshot.last_successful_update_ms;
                    let status = if snapshot.latest_event.status == SourceStatus::Healthy.as_str() {
                        monitor.classify(
                            now_ms,
                            last_successful_update_ms.unwrap_or(snapshot.latest_event.observed_at_ms),
                            state.source_health_stale_after_ms,
                        )
                    } else {
                        source_status_from_str(snapshot.latest_event.status.as_str())
                    };

                    SourceHealthResponse {
                        source_id: snapshot.latest_event.source_id,
                        status: status.as_str().to_owned(),
                        message: snapshot.latest_event.message,
                        observed_at_ms: snapshot.latest_event.observed_at_ms,
                        last_successful_update_ms,
                        age_ms: last_successful_update_ms
                            .map(|observed_at_ms| now_ms.saturating_sub(observed_at_ms)),
                    }
                })
                .collect(),
        ))
    }

    fn open_store(state: &ApiState) -> Result<SqliteMarketDataStore, StatusCode> {
        let store = SqliteMarketDataStore::open(&state.market_data_db_path)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        store
            .apply_migrations()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(store)
    }

    fn parse_timeframe(value: Option<&str>) -> Result<Timeframe, StatusCode> {
        match value {
            Some(timeframe) => timeframe.parse::<Timeframe>().map_err(|_| StatusCode::BAD_REQUEST),
            None => Ok(Timeframe::OneMinute),
        }
    }

    fn normalize_limit(value: Option<usize>, default_limit: usize) -> usize {
        value.unwrap_or(default_limit).clamp(1, 1_000)
    }

    fn current_time_ms() -> Result<i64, std::time::SystemTimeError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
    }

    fn source_status_from_str(value: &str) -> SourceStatus {
        match value {
            "healthy" => SourceStatus::Healthy,
            "degraded" => SourceStatus::Degraded,
            "unavailable" => SourceStatus::Unavailable,
            _ => SourceStatus::Unavailable,
        }
    }

    fn regime_label_as_str(value: scenario_core::MarketRegimeLabel) -> &'static str {
        match value {
            scenario_core::MarketRegimeLabel::Uptrend => "uptrend",
            scenario_core::MarketRegimeLabel::Downtrend => "downtrend",
            scenario_core::MarketRegimeLabel::Range => "range",
            scenario_core::MarketRegimeLabel::HighVolatilityTransition => "high_volatility_transition",
        }
    }

    fn expected_direction_as_str(value: scenario_core::ExpectedDirection) -> &'static str {
        match value {
            scenario_core::ExpectedDirection::Bullish => "bullish",
            scenario_core::ExpectedDirection::Neutral => "neutral",
            scenario_core::ExpectedDirection::Bearish => "bearish",
        }
    }
}

mod tasks {
    use std::fs;
    use std::path::Path;

    use crate::config::AppConfig;
    use market_data_core::instrument::Instrument;
    use market_data_infrastructure::adapters::BinanceMarketDataAdapter;
    use market_data_infrastructure::adapters::BinanceStreamEvent;
    use market_data_core::timeframe::Timeframe;
    use persistence_core::models::{AlertRecord, LatestMarketOverviewRecord};
    use persistence_core::queries::{LatestMarketOverviewQuery, MarketOverviewQueryService};
    use persistence_core::repositories::{
        AlertRepository,
        CandleRepository,
        FeatureSnapshotRepository,
        LivePriceSnapshotRepository,
        RegimeSnapshotRepository,
        ScenarioSnapshotRepository,
        SourceHealthRepository,
    };
    use persistence_core::models::SourceHealthRecord;
    use persistence_core::sqlite::SqliteMarketDataStore;
    use market_data_infrastructure::health::SourceStatus;
    use scenario_core::{
        aggregation::TimeframeDerivationEngine,
        ExpectedDirection,
        FeatureSnapshotBuilder,
        MarketRegimeLabel,
        MarketRegimeStrategy,
        ScenarioExplanationBuilder,
        ScenarioSnapshotFactory,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct BinanceBootstrapIngestionTask;

    #[derive(Debug, Clone, PartialEq)]
    pub struct IngestionSummary {
        pub persisted_candle_count: usize,
        pub snapshot_observed_at_ms: i64,
        pub websocket_stream_url: String,
        pub persisted_analytics_count: usize,
        pub latest_market_overview: Option<LatestMarketOverviewRecord>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LiveSyncSummary {
        pub processed_message_count: usize,
        pub persisted_candle_update_count: usize,
        pub persisted_snapshot_update_count: usize,
        pub persisted_analytics_count: usize,
        pub latest_market_overview: Option<LatestMarketOverviewRecord>,
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct AnalyticsSnapshotPipeline;

    #[derive(Debug, Default, Clone, Copy)]
    pub struct SnapshotAlertFactory;

    pub fn persist_binance_health_event(
        store: &SqliteMarketDataStore,
        status: SourceStatus,
        message: impl Into<String>,
    ) -> Result<(), String> {
        let observed_at_ms = current_time_ms()?;
        let event = SourceHealthRecord {
            source_id: "binance".to_owned(),
            status: status.as_str().to_owned(),
            message: message.into(),
            observed_at_ms,
            created_at_ms: observed_at_ms,
        };

        SourceHealthRepository::save(store, &event).map_err(|error| error.to_string())
    }

    fn current_time_ms() -> Result<i64, String> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
    }

    impl SnapshotAlertFactory {
        pub fn build(
            self,
            previous_regime: Option<&scenario_core::MarketRegimeSnapshot>,
            current_regime: &scenario_core::MarketRegimeSnapshot,
            previous_scenario: Option<&scenario_core::ScenarioSnapshot>,
            current_scenario: &scenario_core::ScenarioSnapshot,
        ) -> Vec<AlertRecord> {
            let mut alerts = Vec::new();

            if let Some(previous_regime) = previous_regime
                && previous_regime.regime_label != current_regime.regime_label
            {
                alerts.push(AlertRecord {
                    instrument_id: current_regime.instrument_id.clone(),
                    timeframe: current_regime.timeframe,
                    alert_type: "regime_changed".to_owned(),
                    severity: regime_change_severity(current_regime.regime_label).to_owned(),
                    message: format!(
                        "Regime changed from {} to {}",
                        regime_label_as_str(previous_regime.regime_label),
                        regime_label_as_str(current_regime.regime_label),
                    ),
                    triggered_at_ms: current_regime.observed_at.0,
                    scenario_snapshot_id: Some(scenario_snapshot_identifier(current_scenario)),
                    is_acknowledged: false,
                });
            }

            if let Some(previous_scenario) = previous_scenario
                && previous_scenario.expected_direction != current_scenario.expected_direction
            {
                alerts.push(AlertRecord {
                    instrument_id: current_scenario.instrument_id.clone(),
                    timeframe: current_scenario.timeframe,
                    alert_type: "scenario_shifted".to_owned(),
                    severity: "warning".to_owned(),
                    message: format!(
                        "Scenario direction shifted from {} to {}",
                        expected_direction_as_str(previous_scenario.expected_direction),
                        expected_direction_as_str(current_scenario.expected_direction),
                    ),
                    triggered_at_ms: current_scenario.observed_at.0,
                    scenario_snapshot_id: Some(scenario_snapshot_identifier(current_scenario)),
                    is_acknowledged: false,
                });
            }

            alerts
        }
    }

    impl AnalyticsSnapshotPipeline {
        pub fn compute_and_persist(
            self,
            store: &SqliteMarketDataStore,
            candles: &[market_data_core::candle::Candle],
        ) -> Result<usize, String> {
            let feature_snapshot = FeatureSnapshotBuilder
                .build(candles)
                .ok_or_else(|| "at least one candle is required to build analytics snapshots".to_owned())?;
            let regime_snapshot = MarketRegimeStrategy.classify(&feature_snapshot);
            let explanation = ScenarioExplanationBuilder.build(&feature_snapshot, &regime_snapshot);
            let scenario_snapshot = ScenarioSnapshotFactory.build(&feature_snapshot, &regime_snapshot, explanation);
            let previous_regime_snapshot = store
                .load_latest_regime_snapshot(&regime_snapshot.instrument_id, regime_snapshot.timeframe)
                .map_err(|error| error.to_string())?;
            let previous_scenario_snapshot = store
                .load_latest_scenario_snapshot(&scenario_snapshot.instrument_id, scenario_snapshot.timeframe)
                .map_err(|error| error.to_string())?;

            FeatureSnapshotRepository::save(store, &feature_snapshot).map_err(|error| error.to_string())?;
            RegimeSnapshotRepository::save(store, &regime_snapshot).map_err(|error| error.to_string())?;
            ScenarioSnapshotRepository::save(store, &scenario_snapshot).map_err(|error| error.to_string())?;

            for alert in SnapshotAlertFactory.build(
                previous_regime_snapshot.as_ref(),
                &regime_snapshot,
                previous_scenario_snapshot.as_ref(),
                &scenario_snapshot,
            ) {
                AlertRepository::save(store, &alert).map_err(|error| error.to_string())?;
            }

            Ok(3)
        }

        pub fn persist_timeframe_views(
            self,
            store: &SqliteMarketDataStore,
            one_minute_candles: &[market_data_core::candle::Candle],
        ) -> Result<usize, String> {
            let derivation_engine = TimeframeDerivationEngine;
            let mut persisted_analytics_count = 0;

            for timeframe in derivation_engine.default_targets() {
                let candles = derivation_engine.derive(one_minute_candles, timeframe);
                if candles.is_empty() {
                    continue;
                }

                for candle in &candles {
                    CandleRepository::save(store, candle).map_err(|error| error.to_string())?;
                }

                persisted_analytics_count += self.compute_and_persist(store, &candles)?;
            }

            Ok(persisted_analytics_count)
        }
    }

    fn scenario_snapshot_identifier(snapshot: &scenario_core::ScenarioSnapshot) -> String {
        format!(
            "{}:{}:{}:{}",
            snapshot.instrument_id,
            snapshot.timeframe,
            snapshot.observed_at.0,
            "v1"
        )
    }

    fn regime_change_severity(label: MarketRegimeLabel) -> &'static str {
        match label {
            MarketRegimeLabel::HighVolatilityTransition => "warning",
            MarketRegimeLabel::Uptrend | MarketRegimeLabel::Downtrend | MarketRegimeLabel::Range => "info",
        }
    }

    fn regime_label_as_str(label: MarketRegimeLabel) -> &'static str {
        match label {
            MarketRegimeLabel::Uptrend => "uptrend",
            MarketRegimeLabel::Downtrend => "downtrend",
            MarketRegimeLabel::Range => "range",
            MarketRegimeLabel::HighVolatilityTransition => "high_volatility_transition",
        }
    }

    fn expected_direction_as_str(direction: ExpectedDirection) -> &'static str {
        match direction {
            ExpectedDirection::Bullish => "bullish",
            ExpectedDirection::Neutral => "neutral",
            ExpectedDirection::Bearish => "bearish",
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct MarketOverviewReadTask;

    impl MarketOverviewReadTask {
        pub fn load_latest(
            self,
            store: &SqliteMarketDataStore,
            instrument: &Instrument,
            timeframe: Timeframe,
        ) -> Result<Option<LatestMarketOverviewRecord>, String> {
            let query = LatestMarketOverviewQuery::for_instrument(instrument.id.clone(), timeframe);

            MarketOverviewQueryService
                .load_latest(store, &query)
                .map_err(|error| error.to_string())
        }
    }

    impl BinanceBootstrapIngestionTask {
        pub fn ingest_from_payloads(
            self,
            config: &AppConfig,
            store: &SqliteMarketDataStore,
            klines_payload: &str,
            market_summary_payload: &str,
        ) -> Result<IngestionSummary, String> {
            let instrument = Instrument::btc_usd_spot();
            let adapter = BinanceMarketDataAdapter;
            let candles = adapter.parse_klines_response(
                &instrument,
                Timeframe::OneMinute,
                klines_payload,
            )?;
            let snapshot = adapter.parse_market_summary_response(&instrument, market_summary_payload)?;
            let subscriptions = adapter.build_primary_stream_subscriptions(&config.primary_exchange_symbol);
            let websocket_stream_url = adapter
                .websocket_client()
                .build_combined_stream_url(&subscriptions)?;

            for candle in &candles {
                CandleRepository::save(store, candle).map_err(|error| error.to_string())?;
            }

            LivePriceSnapshotRepository::save(store, &snapshot).map_err(|error| error.to_string())?;
            let persisted_analytics_count = AnalyticsSnapshotPipeline
                .compute_and_persist(store, &candles)?
                + AnalyticsSnapshotPipeline.persist_timeframe_views(store, &candles)?;
            persist_binance_health_event(
                store,
                SourceStatus::Healthy,
                "Binance REST sync and BTC analytics completed successfully",
            )?;
            let latest_market_overview = MarketOverviewReadTask
                .load_latest(store, &instrument, Timeframe::OneMinute)?;

            Ok(IngestionSummary {
                persisted_candle_count: candles.len(),
                snapshot_observed_at_ms: snapshot.observed_at.0,
                websocket_stream_url,
                persisted_analytics_count,
                latest_market_overview,
            })
        }

        pub fn ingest_bootstrap_cycle_into_store(
            self,
            config: &AppConfig,
            store: &SqliteMarketDataStore,
        ) -> Result<IngestionSummary, String> {
            let adapter = BinanceMarketDataAdapter;
            let rest_client = adapter.rest_client();
            let klines_request =
                adapter.build_klines_request(&config.primary_exchange_symbol, Timeframe::OneMinute, 120, None, None);
            let market_summary_request = adapter.build_market_summary_request(&config.primary_exchange_symbol);

            let klines_payload = rest_client.execute_text(&klines_request)?.body;
            let market_summary_payload = rest_client.execute_text(&market_summary_request)?.body;

            self.ingest_from_payloads(config, store, &klines_payload, &market_summary_payload)
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct BinanceLiveSyncTask;

    impl BinanceLiveSyncTask {
        pub fn persist_stream_payloads(
            self,
            _config: &AppConfig,
            store: &SqliteMarketDataStore,
            payloads: &[&str],
        ) -> Result<LiveSyncSummary, String> {
            let instrument = Instrument::btc_usd_spot();
            let adapter = BinanceMarketDataAdapter;
            let mut persisted_candle_update_count = 0;
            let mut persisted_snapshot_update_count = 0;
            let mut latest_candles = Vec::new();

            for payload in payloads {
                match adapter.parse_combined_stream_payload(&instrument, payload)? {
                    BinanceStreamEvent::Kline(candle) => {
                        CandleRepository::save(store, &candle).map_err(|error| error.to_string())?;
                        persisted_candle_update_count += 1;
                        latest_candles.push(candle);
                    }
                    BinanceStreamEvent::MiniTicker(snapshot) => {
                        LivePriceSnapshotRepository::save(store, &snapshot)
                            .map_err(|error| error.to_string())?;
                        persisted_snapshot_update_count += 1;
                    }
                }
            }

            let persisted_analytics_count = if latest_candles.is_empty() {
                0
            } else {
                let candles = store
                    .load_recent_candles(&instrument.id, Timeframe::OneMinute, 120)
                    .map_err(|error| error.to_string())?;
                AnalyticsSnapshotPipeline
                    .compute_and_persist(store, &candles)?
                    + AnalyticsSnapshotPipeline.persist_timeframe_views(store, &candles)?
            };
            let (health_status, health_message) = if payloads.is_empty() {
                (
                    SourceStatus::Degraded,
                    "Binance live stream returned no messages during the sync window".to_owned(),
                )
            } else {
                (
                    SourceStatus::Healthy,
                    format!(
                        "Binance live stream sync processed {} messages",
                        payloads.len()
                    ),
                )
            };
            persist_binance_health_event(store, health_status, health_message)?;
            let latest_market_overview = MarketOverviewReadTask
                .load_latest(store, &instrument, Timeframe::OneMinute)?;

            Ok(LiveSyncSummary {
                processed_message_count: payloads.len(),
                persisted_candle_update_count,
                persisted_snapshot_update_count,
                persisted_analytics_count,
                latest_market_overview,
            })
        }

        pub fn consume_live_stream_messages(
            self,
            config: &AppConfig,
            store: &SqliteMarketDataStore,
        ) -> Result<LiveSyncSummary, String> {
            let adapter = BinanceMarketDataAdapter;
            let subscriptions = adapter.build_primary_stream_subscriptions(&config.primary_exchange_symbol);
            let websocket_client = adapter.websocket_client();
            let messages = websocket_client
                .read_text_messages(&subscriptions, config.live_sync_message_limit)?;
            let payloads = messages.iter().map(String::as_str).collect::<Vec<_>>();

            self.persist_stream_payloads(config, store, &payloads)
        }
    }

    pub fn open_market_data_store(config: &AppConfig) -> Result<SqliteMarketDataStore, String> {
        if let Some(parent) = Path::new(&config.market_data_db_path).parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let store = SqliteMarketDataStore::open(&config.market_data_db_path)
            .map_err(|error| error.to_string())?;
        store.apply_migrations().map_err(|error| error.to_string())?;

        Ok(store)
    }
}

mod runtime {
    use std::time::Duration;

    use crate::config::AppConfig;
    use crate::tasks::{
        open_market_data_store,
        persist_binance_health_event,
        BinanceBootstrapIngestionTask,
        BinanceLiveSyncTask,
        IngestionSummary,
        LiveSyncSummary,
    };
    use market_data_infrastructure::health::{SourceHealthMonitor, SourceStatus};

    #[derive(Debug, Clone, PartialEq)]
    pub struct RuntimeSummary {
        pub bootstrap: IngestionSummary,
        pub live_sync: LiveSyncSummary,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Runtime {
        pub config: AppConfig,
    }

    impl Runtime {
        pub fn run(&self) -> Result<RuntimeSummary, String> {
            let store = open_market_data_store(&self.config)?;
            let bootstrap = BinanceBootstrapIngestionTask
                .ingest_bootstrap_cycle_into_store(&self.config, &store)?;
            let live_sync = BinanceLiveSyncTask.consume_live_stream_messages(&self.config, &store)?;

            Ok(RuntimeSummary {
                bootstrap,
                live_sync,
            })
        }
    }

    pub async fn run_sync_cycle(config: &AppConfig) -> Result<RuntimeSummary, String> {
        let config = config.clone();

        tokio::task::spawn_blocking(move || {
            let result = Runtime { config: config.clone() }.run();
            if let Err(error) = &result {
                record_binance_sync_failure(&config, error);
            }
            result
        })
            .await
            .map_err(|error| format!("market data sync task failed: {error}"))?
    }

    fn record_binance_sync_failure(config: &AppConfig, error: &str) {
        let Ok(store) = open_market_data_store(config) else {
            return;
        };

        let now_ms = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_millis().min(i64::MAX as u128) as i64,
            Err(_) => return,
        };
        let last_successful_update_ms = store
            .load_source_health_snapshots()
            .ok()
            .and_then(|snapshots| {
                snapshots
                    .into_iter()
                    .find(|snapshot| snapshot.latest_event.source_id == "binance")
                    .and_then(|snapshot| snapshot.last_successful_update_ms)
            });
        let classified_status = last_successful_update_ms
            .map(|observed_at_ms| {
                SourceHealthMonitor.classify(
                    now_ms,
                    observed_at_ms,
                    config.source_health_stale_after_ms(),
                )
            })
            .unwrap_or(SourceStatus::Unavailable);
        let status = match classified_status {
            SourceStatus::Healthy | SourceStatus::Degraded => SourceStatus::Degraded,
            SourceStatus::Unavailable => SourceStatus::Unavailable,
        };

        let _ = persist_binance_health_event(
            &store,
            status,
            format!("Binance sync failed: {error}"),
        );
    }

    pub fn spawn_periodic_sync(config: AppConfig) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.api_sync_interval_secs));

            interval.tick().await;

            loop {
                interval.tick().await;

                if let Err(error) = run_sync_cycle(&config).await {
                    eprintln!("periodic market data sync failed: {error}");
                }
            }
        });
    }
}

mod bootstrap {
    use crate::config::AppConfig;
    use crate::runtime::Runtime;

    #[derive(Debug, Default, Clone, Copy)]
    pub struct Bootstrap;

    impl Bootstrap {
        pub fn build(self, config: &AppConfig) -> Runtime {
            Runtime {
                config: config.clone(),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let config = config::AppConfig::default();
    if std::env::args().nth(1).as_deref() == Some("serve-api") {
        if let Err(error) = api::serve(&config).await {
            eprintln!("api server failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    let runtime = bootstrap::Bootstrap.build(&config);

    match runtime.run() {
        Ok(summary) => {
            if let Some(overview) = summary.live_sync.latest_market_overview.as_ref() {
                println!(
                    "persisted {} candles, latest snapshot at {}, live streams {}, processed {} live messages, regime {:?}, direction {:?}, last price {:.2}",
                    summary.bootstrap.persisted_candle_count,
                    summary.bootstrap.snapshot_observed_at_ms,
                    summary.bootstrap.websocket_stream_url,
                    summary.live_sync.processed_message_count,
                    overview.regime_snapshot.regime_label,
                    overview.scenario_snapshot.expected_direction,
                    overview.live_price_snapshot.last_price.0,
                );
            } else {
                println!(
                    "persisted {} candles, latest snapshot at {}, live streams {}, processed {} live messages",
                    summary.bootstrap.persisted_candle_count,
                    summary.bootstrap.snapshot_observed_at_ms,
                    summary.bootstrap.websocket_stream_url,
                    summary.live_sync.processed_message_count,
                );
            }
        }
        Err(error) => {
            eprintln!("bootstrap ingestion failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::State;
    use super::config::AppConfig;
    use super::api::{
        get_alerts,
        get_candles,
        get_market_overview,
        get_scenario_history,
        get_source_health,
        ApiListQuery,
        ApiState,
    };
    use super::tasks::{BinanceBootstrapIngestionTask, BinanceLiveSyncTask};
    use axum::extract::Query;
    use persistence_core::repositories::AlertHistoryQueryRepository;
    use persistence_core::repositories::RegimeSnapshotRepository;
    use persistence_core::repositories::SourceHealthRepository;
    use persistence_core::sqlite::SqliteMarketDataStore;
    use persistence_core::repositories::ScenarioSnapshotRepository;
    use persistence_core::models::SourceHealthRecord;
    use rusqlite::{params, Connection};
    use scenario_core::{ExpectedDirection, MarketRegimeLabel};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path(test_name: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("{test_name}-{unique}.sqlite3"));
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn ingests_payloads_into_sqlite_store() {
        let config = AppConfig::default();
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let summary = BinanceBootstrapIngestionTask
            .ingest_from_payloads(
                &config,
                &store,
                r#"[
                    [1710000000000,"68000.00","68500.00","67950.00","68450.12","123.45",1710000059999,"0",42,"0","0","0"],
                    [1710000060000,"68450.12","68600.00","68400.00","68500.00","95.00",1710000119999,"0",31,"0","0","0"]
                ]"#,
                r#"{
                    "symbol":"BTCUSDT",
                    "priceChangePercent":"3.12",
                    "lastPrice":"68500.00",
                    "volume":"8913.3",
                    "closeTime":1710000120000
                }"#,
            )
            .expect("bootstrap ingestion should succeed");

        assert_eq!(summary.persisted_candle_count, 2);
        assert_eq!(summary.snapshot_observed_at_ms, 1_710_000_120_000);
        assert_eq!(summary.persisted_analytics_count, 21);
        assert_eq!(
            summary
                .latest_market_overview
                .as_ref()
                .map(|overview| overview.regime_snapshot.regime_label),
            Some(MarketRegimeLabel::Uptrend)
        );
        assert_eq!(
            summary.websocket_stream_url,
            "wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/btcusdt@miniTicker"
        );
        assert_eq!(store.count_rows("candles").unwrap(), 8);
        assert_eq!(store.count_rows("live_price_snapshots").unwrap(), 1);
        assert_eq!(store.count_rows("feature_snapshots").unwrap(), 7);
        assert_eq!(store.count_rows("regime_snapshots").unwrap(), 7);
        assert_eq!(store.count_rows("scenario_snapshots").unwrap(), 7);
        assert_eq!(store.count_rows("source_health_events").unwrap(), 1);
    }

    #[test]
    fn persists_live_stream_payloads_into_sqlite_store() {
        let config = AppConfig::default();
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let summary = BinanceLiveSyncTask
            .persist_stream_payloads(
                &config,
                &store,
                &[
                    r#"{
                        "stream":"btcusdt@kline_1m",
                        "data":{
                            "e":"kline",
                            "E":1710000120000,
                            "s":"BTCUSDT",
                            "k":{
                                "t":1710000060000,
                                "T":1710000119999,
                                "s":"BTCUSDT",
                                "i":"1m",
                                "o":"68450.12",
                                "c":"68500.00",
                                "h":"68600.00",
                                "l":"68400.00",
                                "v":"95.00",
                                "n":31,
                                "x":false
                            }
                        }
                    }"#,
                    r#"{
                        "stream":"btcusdt@miniTicker",
                        "data":{
                            "e":"24hrMiniTicker",
                            "E":1710000120000,
                            "s":"BTCUSDT",
                            "c":"68500.00",
                            "o":"68000.00",
                            "h":"68600.00",
                            "l":"67900.00",
                            "v":"8913.3",
                            "q":"0"
                        }
                    }"#,
                ],
            )
            .expect("live sync persistence should succeed");

        assert_eq!(summary.processed_message_count, 2);
        assert_eq!(summary.persisted_candle_update_count, 1);
        assert_eq!(summary.persisted_snapshot_update_count, 1);
        assert_eq!(summary.persisted_analytics_count, 21);
        assert_eq!(
            summary
                .latest_market_overview
                .as_ref()
                .map(|overview| overview.scenario_snapshot.expected_direction),
            Some(ExpectedDirection::Bullish)
        );
        assert_eq!(store.count_rows("candles").unwrap(), 7);
        assert_eq!(store.count_rows("live_price_snapshots").unwrap(), 1);
        assert_eq!(store.count_rows("feature_snapshots").unwrap(), 7);
        assert_eq!(store.count_rows("regime_snapshots").unwrap(), 7);
        assert_eq!(store.count_rows("scenario_snapshots").unwrap(), 7);
        assert_eq!(store.count_rows("source_health_events").unwrap(), 1);
    }

    #[test]
    fn generates_alerts_when_snapshot_state_changes() {
        let config = AppConfig::default();
        let store = SqliteMarketDataStore::open_in_memory().expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let previous_regime_snapshot = scenario_core::MarketRegimeSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            timeframe: market_data_core::timeframe::Timeframe::OneMinute,
            observed_at: market_data_core::value_objects::Timestamp::new(1_710_000_000_000).unwrap(),
            regime_label: scenario_core::MarketRegimeLabel::Downtrend,
            regime_score: 400.0,
        };
        let previous_scenario_snapshot = scenario_core::ScenarioSnapshot {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            timeframe: market_data_core::timeframe::Timeframe::OneMinute,
            observed_at: market_data_core::value_objects::Timestamp::new(1_710_000_000_000).unwrap(),
            bull_probability: 0.15,
            base_probability: 0.30,
            bear_probability: 0.55,
            trigger_level: 150.0,
            invalidation_level: 100.0,
            expected_direction: scenario_core::ExpectedDirection::Bearish,
            explanation: "BTC scenario was bearish".to_owned(),
        };

        RegimeSnapshotRepository::save(&store, &previous_regime_snapshot)
            .expect("previous regime snapshot should save");
        ScenarioSnapshotRepository::save(&store, &previous_scenario_snapshot)
            .expect("previous scenario snapshot should save");

        let summary = BinanceBootstrapIngestionTask
            .ingest_from_payloads(
                &config,
                &store,
                r#"[
                    [1710000000000,"68000.00","68500.00","67950.00","68450.12","123.45",1710000059999,"0",42,"0","0","0"],
                    [1710000060000,"68450.12","68600.00","68400.00","68500.00","95.00",1710000119999,"0",31,"0","0","0"]
                ]"#,
                r#"{
                    "symbol":"BTCUSDT",
                    "priceChangePercent":"3.12",
                    "lastPrice":"68500.00",
                    "volume":"8913.3",
                    "closeTime":1710000120000
                }"#,
            )
            .expect("bootstrap ingestion should succeed");

        assert_eq!(summary.persisted_analytics_count, 21);
        let observed_at_ms = summary
            .latest_market_overview
            .as_ref()
            .expect("latest market overview should exist")
            .scenario_snapshot
            .observed_at
            .0;

        let alerts = AlertHistoryQueryRepository::load_alert_history(
            &store,
            "BTC-USD-SPOT",
            market_data_core::timeframe::Timeframe::OneMinute,
            10,
        )
        .expect("alert history should load");

        assert_eq!(alerts.len(), 2);
        assert!(alerts.iter().any(|alert| alert.alert_type == "regime_changed"));
        assert!(alerts.iter().any(|alert| alert.alert_type == "scenario_shifted"));
        assert!(alerts.iter().all(|alert| alert.triggered_at_ms == observed_at_ms));
        assert!(alerts.iter().all(|alert| !alert.is_acknowledged));
        assert!(alerts.iter().all(|alert| alert.scenario_snapshot_id.as_deref() == Some(format!("BTC-USD-SPOT:1m:{observed_at_ms}:v1").as_str())));
        assert!(alerts.iter().any(|alert| alert.message == "Regime changed from downtrend to uptrend"));
        assert!(alerts.iter().any(|alert| alert.message == "Scenario direction shifted from bearish to bullish"));
    }

    #[tokio::test]
    async fn returns_market_overview_from_api_handler() {
        let db_path = temp_db_path("market-overview-handler");
        let config = AppConfig {
            market_data_db_path: db_path.clone(),
            ..AppConfig::default()
        };
        let store = SqliteMarketDataStore::open(&db_path).expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        BinanceBootstrapIngestionTask
            .ingest_from_payloads(
                &config,
                &store,
                r#"[
                    [1710000000000,"68000.00","68500.00","67950.00","68450.12","123.45",1710000059999,"0",42,"0","0","0"],
                    [1710000060000,"68450.12","68600.00","68400.00","68500.00","95.00",1710000119999,"0",31,"0","0","0"]
                ]"#,
                r#"{
                    "symbol":"BTCUSDT",
                    "priceChangePercent":"3.12",
                    "lastPrice":"68500.00",
                    "volume":"8913.3",
                    "closeTime":1710000120000
                }"#,
            )
            .expect("bootstrap ingestion should succeed");

        let response = get_market_overview(
            State(ApiState::from_config(&config)),
            Query(ApiListQuery {
                timeframe: None,
                limit: None,
            }),
        )
        .await
            .expect("api handler should return market overview");

        assert_eq!(response.0.instrument_id, "BTC-USD-SPOT");
        assert_eq!(response.0.timeframe, "1m");
        assert_eq!(response.0.last_price, 68_500.0);
        assert_eq!(response.0.support_level, 67_950.0);
        assert_eq!(response.0.resistance_level, 68_600.0);
        assert_eq!(response.0.trigger_level, 68_600.0);
        assert_eq!(response.0.invalidation_level, 67_950.0);
        assert_eq!(response.0.regime_label, "uptrend");
        assert_eq!(response.0.expected_direction, "bullish");

        let higher_timeframe_response = get_market_overview(
            State(ApiState::from_config(&config)),
            Query(ApiListQuery {
                timeframe: Some("5m".to_owned()),
                limit: None,
            }),
        )
        .await
        .expect("api handler should return higher timeframe market overview");

        assert_eq!(higher_timeframe_response.0.timeframe, "5m");

        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn returns_degraded_source_health_when_last_success_is_stale() {
        let db_path = temp_db_path("source-health-handler");
        let config = AppConfig {
            market_data_db_path: db_path.clone(),
            source_health_stale_after_secs: 1,
            ..AppConfig::default()
        };
        let store = SqliteMarketDataStore::open(&db_path).expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_millis() as i64;

        SourceHealthRepository::save(
            &store,
            &SourceHealthRecord {
                source_id: "binance".to_owned(),
                status: "healthy".to_owned(),
                message: "Binance sync succeeded".to_owned(),
                observed_at_ms: now_ms.saturating_sub(2_000),
                created_at_ms: now_ms.saturating_sub(2_000),
            },
        )
        .expect("source health event should save");

        let response = get_source_health(State(ApiState::from_config(&config)))
            .await
            .expect("source health handler should succeed");

        assert_eq!(response.0.len(), 1);
        assert_eq!(response.0[0].source_id, "binance");
        assert_eq!(response.0[0].status, "degraded");
        assert!(response.0[0].age_ms.is_some());

        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn returns_recent_candles_from_api_handler() {
        let db_path = temp_db_path("candles-handler");
        let config = AppConfig {
            market_data_db_path: db_path.clone(),
            ..AppConfig::default()
        };
        let store = SqliteMarketDataStore::open(&db_path).expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        BinanceBootstrapIngestionTask
            .ingest_from_payloads(
                &config,
                &store,
                r#"[
                    [1710000000000,"68000.00","68500.00","67950.00","68450.12","123.45",1710000059999,"0",42,"0","0","0"],
                    [1710000060000,"68450.12","68600.00","68400.00","68500.00","95.00",1710000119999,"0",31,"0","0","0"]
                ]"#,
                r#"{
                    "symbol":"BTCUSDT",
                    "priceChangePercent":"3.12",
                    "lastPrice":"68500.00",
                    "volume":"8913.3",
                    "closeTime":1710000120000
                }"#,
            )
            .expect("bootstrap ingestion should succeed");

        let response = get_candles(
            State(ApiState::from_config(&config)),
            Query(ApiListQuery {
                timeframe: Some("1m".to_owned()),
                limit: Some(2),
            }),
        )
        .await
        .expect("api handler should return candles");

        assert_eq!(response.0.len(), 2);
        assert_eq!(response.0[0].open_time_ms, 1_710_000_000_000);
        assert_eq!(response.0[1].close, 68_500.0);

        let higher_timeframe_response = get_candles(
            State(ApiState::from_config(&config)),
            Query(ApiListQuery {
                timeframe: Some("5m".to_owned()),
                limit: Some(1),
            }),
        )
        .await
        .expect("api handler should return higher timeframe candles");

        assert_eq!(higher_timeframe_response.0.len(), 1);
        assert_eq!(higher_timeframe_response.0[0].timeframe, "5m");
        assert_eq!(higher_timeframe_response.0[0].open, 68_000.0);
        assert_eq!(higher_timeframe_response.0[0].close, 68_500.0);

        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn returns_scenario_history_from_api_handler() {
        let db_path = temp_db_path("scenario-history-handler");
        let config = AppConfig {
            market_data_db_path: db_path.clone(),
            ..AppConfig::default()
        };
        let store = SqliteMarketDataStore::open(&db_path).expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        for observed_at in [1_710_000_060_000_i64, 1_710_000_120_000_i64] {
            let scenario_snapshot = scenario_core::ScenarioSnapshot {
                instrument_id: "BTC-USD-SPOT".to_owned(),
                timeframe: market_data_core::timeframe::Timeframe::OneMinute,
                observed_at: market_data_core::value_objects::Timestamp::new(observed_at).unwrap(),
                bull_probability: 0.55,
                base_probability: 0.30,
                bear_probability: 0.15,
                trigger_level: 150.0,
                invalidation_level: 100.0,
                expected_direction: scenario_core::ExpectedDirection::Bullish,
                explanation: format!("BTC scenario at {observed_at}"),
            };

            ScenarioSnapshotRepository::save(&store, &scenario_snapshot).expect("scenario snapshot should save");
        }

        let response = get_scenario_history(
            State(ApiState::from_config(&config)),
            Query(ApiListQuery {
                timeframe: Some("1m".to_owned()),
                limit: Some(2),
            }),
        )
        .await
        .expect("api handler should return scenario history");

        assert_eq!(response.0.len(), 2);
        assert_eq!(response.0[0].observed_at_ms, 1_710_000_060_000);
        assert_eq!(response.0[1].expected_direction, "bullish");

        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn returns_alert_history_from_api_handler() {
        let db_path = temp_db_path("alert-history-handler");
        let config = AppConfig {
            market_data_db_path: db_path.clone(),
            ..AppConfig::default()
        };
        let store = SqliteMarketDataStore::open(&db_path).expect("sqlite store should open");
        store.apply_migrations().expect("migrations should apply");

        let connection = Connection::open(&db_path).expect("sqlite connection should open");
        for (id, triggered_at_ms, alert_type, severity, message, is_acknowledged) in [
            (
                "alert-1",
                1_710_000_060_000_i64,
                "regime_changed",
                "info",
                "Regime changed to uptrend",
                0_i64,
            ),
            (
                "alert-2",
                1_710_000_120_000_i64,
                "scenario_shifted",
                "warning",
                "Scenario shifted bullish",
                1_i64,
            ),
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
                        alert_type,
                        severity,
                        message,
                        triggered_at_ms,
                        is_acknowledged,
                        triggered_at_ms,
                    ],
                )
                .expect("alert row should insert");
        }

        let response = get_alerts(
            State(ApiState::from_config(&config)),
            Query(ApiListQuery {
                timeframe: Some("1m".to_owned()),
                limit: Some(2),
            }),
        )
        .await
        .expect("api handler should return alerts");

        assert_eq!(response.0.len(), 2);
        assert_eq!(response.0[0].triggered_at_ms, 1_710_000_060_000);
        assert_eq!(response.0[0].alert_type, "regime_changed");
        assert_eq!(response.0[1].severity, "warning");
        assert!(response.0[1].is_acknowledged);

        let _ = std::fs::remove_file(db_path);
    }
}
