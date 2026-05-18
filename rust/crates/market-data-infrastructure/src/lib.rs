use market_data_core::instrument::Instrument;
use market_data_core::timeframe::Timeframe;
use market_data_core::value_objects::Timestamp;

pub mod clients {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RestMarketDataClient {
        pub base_url: String,
    }

    impl RestMarketDataClient {
        pub fn new(base_url: impl Into<String>) -> Self {
            Self {
                base_url: base_url.into(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WebSocketMarketDataClient {
        pub base_url: String,
    }

    impl WebSocketMarketDataClient {
        pub fn new(base_url: impl Into<String>) -> Self {
            Self {
                base_url: base_url.into(),
            }
        }
    }
}

pub mod adapters {
    #[derive(Debug, Default, Clone, Copy)]
    pub struct BinanceMarketDataAdapter;

    impl BinanceMarketDataAdapter {
        pub fn historical_klines_path(self) -> &'static str {
            "/api/v3/klines"
        }

        pub fn latest_price_path(self) -> &'static str {
            "/api/v3/ticker/price"
        }

        pub fn mini_ticker_stream_name(self, symbol: &str) -> String {
            format!("{}@miniTicker", symbol.to_ascii_lowercase())
        }

        pub fn kline_stream_name(self, symbol: &str, timeframe: &str) -> String {
            format!("{}@kline_{}", symbol.to_ascii_lowercase(), timeframe)
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct KrakenMarketDataAdapter;

    impl KrakenMarketDataAdapter {
        pub fn ohlc_path(self) -> &'static str {
            "/0/public/OHLC"
        }

        pub fn ticker_channel(self) -> &'static str {
            "ticker"
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct CoinGeckoMarketDataAdapter;

    impl CoinGeckoMarketDataAdapter {
        pub fn simple_price_path(self) -> &'static str {
            "/api/v3/simple/price"
        }

        pub fn coins_markets_path(self) -> &'static str {
            "/api/v3/coins/markets"
        }
    }
}

pub mod normalization {
    use super::{Instrument, Timeframe, Timestamp};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct SymbolMapper;

    impl SymbolMapper {
        pub fn normalize_binance_spot(self, symbol: &str) -> Option<Instrument> {
            match symbol {
                "BTCUSDT" => Some(Instrument::btc_usd_spot()),
                _ => None,
            }
        }

        pub fn normalize_kraken_spot(self, pair: &str) -> Option<Instrument> {
            match pair {
                "XBTUSD" | "BTC/USD" => Some(Instrument::btc_usd_spot()),
                _ => None,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct TimestampNormalizer;

    impl TimestampNormalizer {
        pub fn from_milliseconds(self, value: i64) -> Option<Timestamp> {
            Timestamp::new(value).ok()
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct MarketEventNormalizer;

    impl MarketEventNormalizer {
        pub fn normalize_kline_window(
            self,
            open_time_ms: i64,
            close_time_ms: i64,
            timeframe: Timeframe,
        ) -> Option<(Timestamp, Timestamp, Timeframe)> {
            let start = Timestamp::new(open_time_ms).ok()?;
            let end = Timestamp::new(close_time_ms).ok()?;
            Some((start, end, timeframe))
        }
    }
}

pub mod health {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SourceStatus {
        Healthy,
        Degraded,
        Unavailable,
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct SourceHealthMonitor;

    impl SourceHealthMonitor {
        pub fn classify(
            self,
            now_ms: i64,
            last_successful_update_ms: i64,
            stale_after_ms: i64,
        ) -> SourceStatus {
            let age = now_ms.saturating_sub(last_successful_update_ms);
            if age <= stale_after_ms {
                return SourceStatus::Healthy;
            }

            if age <= stale_after_ms.saturating_mul(3) {
                return SourceStatus::Degraded;
            }

            SourceStatus::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::adapters::BinanceMarketDataAdapter;
    use super::health::{SourceHealthMonitor, SourceStatus};
    use super::normalization::SymbolMapper;

    #[test]
    fn maps_primary_binance_symbol() {
        let mapper = SymbolMapper;
        let instrument = mapper.normalize_binance_spot("BTCUSDT");
        assert!(instrument.is_some());
    }

    #[test]
    fn lowercases_binance_stream_names() {
        let adapter = BinanceMarketDataAdapter;
        assert_eq!(adapter.kline_stream_name("BTCUSDT", "1m"), "btcusdt@kline_1m");
    }

    #[test]
    fn marks_stale_source_as_degraded() {
        let monitor = SourceHealthMonitor;
        let status = monitor.classify(10_000, 5_000, 2_000);
        assert_eq!(status, SourceStatus::Degraded);
    }
}
