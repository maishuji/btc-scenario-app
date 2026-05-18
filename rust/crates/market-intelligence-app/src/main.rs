mod config {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AppConfig {
        pub instrument_symbol: String,
        pub primary_exchange_symbol: String,
        pub market_data_db_path: String,
    }

    impl Default for AppConfig {
        fn default() -> Self {
            Self {
                instrument_symbol: "BTC-USD-SPOT".to_owned(),
                primary_exchange_symbol: "BTCUSDT".to_owned(),
                market_data_db_path: "var/market-data.sqlite3".to_owned(),
            }
        }
    }
}

mod tasks {
    use std::fs;
    use std::path::Path;

    use crate::config::AppConfig;
    use market_data_core::instrument::Instrument;
    use market_data_infrastructure::adapters::BinanceMarketDataAdapter;
    use market_data_core::timeframe::Timeframe;
    use persistence_core::repositories::{CandleRepository, LivePriceSnapshotRepository};
    use persistence_core::sqlite::SqliteMarketDataStore;

    #[derive(Debug, Default, Clone, Copy)]
    pub struct BinanceBootstrapIngestionTask;

    #[derive(Debug, Clone, PartialEq)]
    pub struct IngestionSummary {
        pub persisted_candle_count: usize,
        pub snapshot_observed_at_ms: i64,
        pub websocket_stream_url: String,
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

            Ok(IngestionSummary {
                persisted_candle_count: candles.len(),
                snapshot_observed_at_ms: snapshot.observed_at.0,
                websocket_stream_url,
            })
        }

        pub fn ingest_bootstrap_cycle(self, config: &AppConfig) -> Result<IngestionSummary, String> {
            if let Some(parent) = Path::new(&config.market_data_db_path).parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }

            let store = SqliteMarketDataStore::open(&config.market_data_db_path)
                .map_err(|error| error.to_string())?;
            store.apply_migrations().map_err(|error| error.to_string())?;

            let adapter = BinanceMarketDataAdapter;
            let rest_client = adapter.rest_client();
            let klines_request =
                adapter.build_klines_request(&config.primary_exchange_symbol, Timeframe::OneMinute, 120, None, None);
            let market_summary_request = adapter.build_market_summary_request(&config.primary_exchange_symbol);

            let klines_payload = rest_client.execute_text(&klines_request)?.body;
            let market_summary_payload = rest_client.execute_text(&market_summary_request)?.body;

            self.ingest_from_payloads(config, &store, &klines_payload, &market_summary_payload)
        }
    }
}

mod runtime {
    use crate::config::AppConfig;
    use crate::tasks::{BinanceBootstrapIngestionTask, IngestionSummary};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Runtime {
        pub config: AppConfig,
    }

    impl Runtime {
        pub fn run(&self) -> Result<IngestionSummary, String> {
            BinanceBootstrapIngestionTask.ingest_bootstrap_cycle(&self.config)
        }
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

fn main() {
    let config = config::AppConfig::default();
    let runtime = bootstrap::Bootstrap.build(&config);

    match runtime.run() {
        Ok(summary) => {
            println!(
                "persisted {} candles, latest snapshot at {}, live streams {}",
                summary.persisted_candle_count,
                summary.snapshot_observed_at_ms,
                summary.websocket_stream_url,
            );
        }
        Err(error) => {
            eprintln!("bootstrap ingestion failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::config::AppConfig;
    use super::tasks::BinanceBootstrapIngestionTask;
    use persistence_core::sqlite::SqliteMarketDataStore;

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
        assert_eq!(
            summary.websocket_stream_url,
            "wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/btcusdt@miniTicker"
        );
        assert_eq!(store.count_rows("candles").unwrap(), 2);
        assert_eq!(store.count_rows("live_price_snapshots").unwrap(), 1);
    }
}
