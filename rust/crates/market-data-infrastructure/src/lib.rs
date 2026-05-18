use market_data_core::candle::Candle;
use market_data_core::instrument::Instrument;
use market_data_core::snapshot::LivePriceSnapshot;
use market_data_core::timeframe::Timeframe;
use market_data_core::value_objects::{Price, Timestamp, Volume};
use serde_json::Value;

pub mod clients {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RestRequest {
        pub path: &'static str,
        pub query: Vec<(String, String)>,
    }

    impl RestRequest {
        pub fn query_value(&self, key: &str) -> Option<&str> {
            self.query
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, value)| value.as_str())
        }
    }

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
    pub struct StreamSubscription {
        pub stream_name: String,
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
    use super::clients::{RestMarketDataClient, RestRequest, StreamSubscription, WebSocketMarketDataClient};
    use super::{parse_array_f64, parse_array_i64, parse_json, parse_string_f64, parse_string_i64};
    use super::{Candle, Instrument, LivePriceSnapshot, Price, Timeframe, Timestamp, Volume, Value};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct BinanceMarketDataAdapter;

    impl BinanceMarketDataAdapter {
        pub fn rest_client(self) -> RestMarketDataClient {
            RestMarketDataClient::new("https://api.binance.com")
        }

        pub fn websocket_client(self) -> WebSocketMarketDataClient {
            WebSocketMarketDataClient::new("wss://stream.binance.com:9443")
        }

        pub fn historical_klines_path(self) -> &'static str {
            "/api/v3/klines"
        }

        pub fn latest_price_path(self) -> &'static str {
            "/api/v3/ticker/price"
        }

        pub fn market_summary_path(self) -> &'static str {
            "/api/v3/ticker/24hr"
        }

        pub fn build_klines_request(
            self,
            symbol: &str,
            timeframe: Timeframe,
            limit: u16,
            start_time_ms: Option<i64>,
            end_time_ms: Option<i64>,
        ) -> RestRequest {
            let mut query = vec![
                ("symbol".to_owned(), symbol.to_owned()),
                ("interval".to_owned(), timeframe.as_str().to_owned()),
                ("limit".to_owned(), limit.to_string()),
            ];

            if let Some(start_time_ms) = start_time_ms {
                query.push(("startTime".to_owned(), start_time_ms.to_string()));
            }

            if let Some(end_time_ms) = end_time_ms {
                query.push(("endTime".to_owned(), end_time_ms.to_string()));
            }

            RestRequest {
                path: self.historical_klines_path(),
                query,
            }
        }

        pub fn build_latest_price_request(self, symbol: &str) -> RestRequest {
            RestRequest {
                path: self.latest_price_path(),
                query: vec![("symbol".to_owned(), symbol.to_owned())],
            }
        }

        pub fn build_market_summary_request(self, symbol: &str) -> RestRequest {
            RestRequest {
                path: self.market_summary_path(),
                query: vec![("symbol".to_owned(), symbol.to_owned())],
            }
        }

        pub fn mini_ticker_stream_name(self, symbol: &str) -> String {
            format!("{}@miniTicker", symbol.to_ascii_lowercase())
        }

        pub fn kline_stream_name(self, symbol: &str, timeframe: &str) -> String {
            format!("{}@kline_{}", symbol.to_ascii_lowercase(), timeframe)
        }

        pub fn build_primary_stream_subscriptions(self, symbol: &str) -> Vec<StreamSubscription> {
            vec![
                StreamSubscription {
                    stream_name: self.kline_stream_name(symbol, Timeframe::OneMinute.as_str()),
                },
                StreamSubscription {
                    stream_name: self.mini_ticker_stream_name(symbol),
                },
            ]
        }

        pub fn parse_latest_price_response(self, payload: &str) -> Result<Price, String> {
            let document = parse_json(payload)?;
            let price = parse_string_f64(&document, "price")?;
            Price::new(price).map_err(str::to_owned)
        }

        pub fn parse_market_summary_response(
            self,
            instrument: &Instrument,
            payload: &str,
        ) -> Result<LivePriceSnapshot, String> {
            let document = parse_json(payload)?;
            let last_price = Price::new(parse_string_f64(&document, "lastPrice")?)
                .map_err(str::to_owned)?;
            let volume_24h = Volume::new(parse_string_f64(&document, "volume")?)
                .map_err(str::to_owned)?;
            let observed_at = Timestamp::new(parse_string_i64(&document, "closeTime")?)
                .map_err(str::to_owned)?;
            let price_change_24h = parse_string_f64(&document, "priceChangePercent")?;

            Ok(LivePriceSnapshot {
                instrument_id: instrument.id.clone(),
                source_id: "binance".to_owned(),
                last_price,
                price_change_24h,
                volume_24h,
                observed_at,
            })
        }

        pub fn parse_kline_record(
            self,
            instrument: &Instrument,
            timeframe: Timeframe,
            record: &Value,
        ) -> Result<Candle, String> {
            Ok(Candle {
                instrument_id: instrument.id.clone(),
                timeframe,
                open_time: Timestamp::new(parse_array_i64(record, 0)?).map_err(str::to_owned)?,
                open: Price::new(parse_array_f64(record, 1)?).map_err(str::to_owned)?,
                high: Price::new(parse_array_f64(record, 2)?).map_err(str::to_owned)?,
                low: Price::new(parse_array_f64(record, 3)?).map_err(str::to_owned)?,
                close: Price::new(parse_array_f64(record, 4)?).map_err(str::to_owned)?,
                volume: Volume::new(parse_array_f64(record, 5)?).map_err(str::to_owned)?,
                close_time: Timestamp::new(parse_array_i64(record, 6)?).map_err(str::to_owned)?,
                trade_count: parse_array_i64(record, 8)? as u64,
                is_final: true,
            })
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

fn parse_json(payload: &str) -> Result<Value, String> {
    serde_json::from_str(payload).map_err(|error| error.to_string())
}

fn value_as_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string field: {field}"))
}

fn parse_string_f64(value: &Value, field: &str) -> Result<f64, String> {
    value_as_str(value, field)?
        .parse::<f64>()
        .map_err(|error| format!("invalid float for {field}: {error}"))
}

fn parse_string_i64(value: &Value, field: &str) -> Result<i64, String> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("missing integer field: {field}"))
}

fn parse_array_f64(value: &Value, index: usize) -> Result<f64, String> {
    value
        .get(index)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string field at index {index}"))?
        .parse::<f64>()
        .map_err(|error| format!("invalid float at index {index}: {error}"))
}

fn parse_array_i64(value: &Value, index: usize) -> Result<i64, String> {
    value
        .get(index)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("missing integer field at index {index}"))
}

#[cfg(test)]
mod tests {
    use super::adapters::BinanceMarketDataAdapter;
    use super::health::{SourceHealthMonitor, SourceStatus};
    use super::normalization::SymbolMapper;
    use market_data_core::instrument::Instrument;
    use market_data_core::timeframe::Timeframe;
    use serde_json::json;

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
    fn builds_required_binance_rest_requests() {
        let adapter = BinanceMarketDataAdapter;
        let request = adapter.build_klines_request("BTCUSDT", Timeframe::OneMinute, 1000, None, None);
        let latest_price_request = adapter.build_latest_price_request("BTCUSDT");
        let market_summary_request = adapter.build_market_summary_request("BTCUSDT");

        assert_eq!(request.path, "/api/v3/klines");
        assert_eq!(request.query_value("symbol"), Some("BTCUSDT"));
        assert_eq!(request.query_value("interval"), Some("1m"));
        assert_eq!(request.query_value("limit"), Some("1000"));
        assert_eq!(latest_price_request.path, "/api/v3/ticker/price");
        assert_eq!(market_summary_request.path, "/api/v3/ticker/24hr");
    }

    #[test]
    fn builds_required_binance_stream_subscriptions() {
        let adapter = BinanceMarketDataAdapter;
        let subscriptions = adapter.build_primary_stream_subscriptions("BTCUSDT");

        assert_eq!(subscriptions.len(), 2);
        assert_eq!(subscriptions[0].stream_name, "btcusdt@kline_1m");
        assert_eq!(subscriptions[1].stream_name, "btcusdt@miniTicker");
    }

    #[test]
    fn parses_latest_price_response() {
        let adapter = BinanceMarketDataAdapter;
        let price = adapter
            .parse_latest_price_response(r#"{"symbol":"BTCUSDT","price":"68450.12"}"#)
            .expect("price response should parse");

        assert_eq!(price.0, 68_450.12);
    }

    #[test]
    fn parses_market_summary_response_into_live_snapshot() {
        let adapter = BinanceMarketDataAdapter;
        let snapshot = adapter
            .parse_market_summary_response(
                &Instrument::btc_usd_spot(),
                r#"{
                    "symbol":"BTCUSDT",
                    "priceChangePercent":"3.12",
                    "lastPrice":"68450.12",
                    "volume":"8913.3",
                    "closeTime":1710000000000
                }"#,
            )
            .expect("market summary should parse");

        assert_eq!(snapshot.instrument_id, "BTC-USD-SPOT");
        assert_eq!(snapshot.source_id, "binance");
        assert_eq!(snapshot.last_price.0, 68_450.12);
        assert_eq!(snapshot.price_change_24h, 3.12);
    }

    #[test]
    fn parses_kline_record_into_candle() {
        let adapter = BinanceMarketDataAdapter;
        let record = json!([
            1710000000000_i64,
            "68000.00",
            "68500.00",
            "67950.00",
            "68450.12",
            "123.45",
            1710000059999_i64,
            "0",
            42,
            "0",
            "0",
            "0"
        ]);
        let candle = adapter
            .parse_kline_record(&Instrument::btc_usd_spot(), Timeframe::OneMinute, &record)
            .expect("kline record should parse");

        assert_eq!(candle.instrument_id, "BTC-USD-SPOT");
        assert_eq!(candle.timeframe, Timeframe::OneMinute);
        assert_eq!(candle.open.0, 68_000.0);
        assert_eq!(candle.close.0, 68_450.12);
        assert_eq!(candle.trade_count, 42);
        assert!(candle.is_final);
    }

    #[test]
    fn marks_stale_source_as_degraded() {
        let monitor = SourceHealthMonitor;
        let status = monitor.classify(10_000, 5_000, 2_000);
        assert_eq!(status, SourceStatus::Degraded);
    }
}
