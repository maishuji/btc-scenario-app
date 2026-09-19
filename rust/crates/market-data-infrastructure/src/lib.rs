use market_data_core::candle::Candle;
use market_data_core::instrument::Instrument;
use market_data_core::snapshot::LivePriceSnapshot;
use market_data_core::timeframe::Timeframe;
use market_data_core::value_objects::{Price, Timestamp, Volume};
use reqwest::blocking::Client as BlockingHttpClient;
use serde_json::Value;
use tungstenite::connect as websocket_connect;
use tungstenite::Message as WebSocketMessage;

pub mod clients {
    use super::BlockingHttpClient;
    use super::{websocket_connect, WebSocketMessage};
    use serde_json::json;

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
    pub struct RestResponse {
        pub status_code: u16,
        pub body: String,
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

        pub fn build_url(&self, request: &RestRequest) -> String {
            let base = format!("{}{}", self.base_url.trim_end_matches('/'), request.path);
            if request.query.is_empty() {
                return base;
            }

            let query = request
                .query
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&");

            format!("{base}?{query}")
        }

        pub fn execute_text(&self, request: &RestRequest) -> Result<RestResponse, String> {
            let url = self.build_url(request);
            let response = BlockingHttpClient::new()
                .get(url)
                .send()
                .map_err(|error| error.to_string())?;
            let status_code = response.status().as_u16();
            let body = response.text().map_err(|error| error.to_string())?;

            if status_code >= 400 {
                return Err(format!("http request failed with status {status_code}: {body}"));
            }

            Ok(RestResponse { status_code, body })
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

        pub fn build_combined_stream_url(&self, subscriptions: &[StreamSubscription]) -> Result<String, String> {
            if subscriptions.is_empty() {
                return Err("at least one websocket stream subscription is required".to_owned());
            }

            let streams = subscriptions
                .iter()
                .map(|subscription| subscription.stream_name.as_str())
                .collect::<Vec<_>>()
                .join("/");

            Ok(format!(
                "{}/stream?streams={streams}",
                self.base_url.trim_end_matches('/')
            ))
        }

        pub fn build_subscribe_message(
            &self,
            subscriptions: &[StreamSubscription],
            request_id: u64,
        ) -> Result<String, String> {
            if subscriptions.is_empty() {
                return Err("at least one websocket stream subscription is required".to_owned());
            }

            let params = subscriptions
                .iter()
                .map(|subscription| subscription.stream_name.clone())
                .collect::<Vec<_>>();
            let payload = json!({
                "method": "SUBSCRIBE",
                "params": params,
                "id": request_id,
            });

            Ok(payload.to_string())
        }

        pub fn read_text_messages(
            &self,
            subscriptions: &[StreamSubscription],
            max_messages: usize,
        ) -> Result<Vec<String>, String> {
            if max_messages == 0 {
                return Ok(Vec::new());
            }

            let url = self.build_combined_stream_url(subscriptions)?;
            let (mut socket, _) = websocket_connect(url).map_err(|error| error.to_string())?;
            let mut messages = Vec::with_capacity(max_messages);

            while messages.len() < max_messages {
                match socket.read().map_err(|error| error.to_string())? {
                    WebSocketMessage::Text(payload) => messages.push(payload.to_string()),
                    WebSocketMessage::Binary(payload) => {
                        let payload = String::from_utf8(payload.to_vec()).map_err(|error| error.to_string())?;
                        messages.push(payload);
                    }
                    WebSocketMessage::Ping(payload) => {
                        socket
                            .send(WebSocketMessage::Pong(payload))
                            .map_err(|error| error.to_string())?;
                    }
                    WebSocketMessage::Close(_) => break,
                    _ => {}
                }
            }

            socket.close(None).map_err(|error| error.to_string())?;

            Ok(messages)
        }
    }
}

pub mod adapters {
    use super::clients::{RestMarketDataClient, RestRequest, StreamSubscription, WebSocketMarketDataClient};
    use super::{parse_array_f64, parse_array_i64, parse_json, parse_string_bool, parse_string_f64, parse_string_i64};
    use super::{Candle, Instrument, LivePriceSnapshot, Price, Timeframe, Timestamp, Volume, Value};

    #[derive(Debug, Clone, PartialEq)]
    pub enum BinanceStreamEvent {
        Kline(Candle),
        MiniTicker(LivePriceSnapshot),
    }

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
                source_id: "binance".to_owned(),
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

        pub fn parse_klines_response(
            self,
            instrument: &Instrument,
            timeframe: Timeframe,
            payload: &str,
        ) -> Result<Vec<Candle>, String> {
            let document = parse_json(payload)?;
            let records = document
                .as_array()
                .ok_or_else(|| "expected kline response array".to_owned())?;

            records
                .iter()
                .map(|record| self.parse_kline_record(instrument, timeframe, record))
                .collect()
        }

        pub fn parse_combined_stream_payload(
            self,
            instrument: &Instrument,
            payload: &str,
        ) -> Result<BinanceStreamEvent, String> {
            let document = parse_json(payload)?;
            let stream_name = document
                .get("stream")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing stream field".to_owned())?;
            let data = document
                .get("data")
                .ok_or_else(|| "missing data field".to_owned())?;

            if stream_name.contains("@kline_") {
                let kline = data
                    .get("k")
                    .ok_or_else(|| "missing kline payload".to_owned())?;
                let timeframe = Timeframe::from_stream_id(
                    kline
                        .get("i")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "missing kline interval".to_owned())?,
                )?;

                let candle = Candle {
                    instrument_id: instrument.id.clone(),
                    source_id: "binance".to_owned(),
                    timeframe,
                    open_time: Timestamp::new(parse_string_i64(kline, "t")?).map_err(str::to_owned)?,
                    close_time: Timestamp::new(parse_string_i64(kline, "T")?).map_err(str::to_owned)?,
                    open: Price::new(parse_string_f64(kline, "o")?).map_err(str::to_owned)?,
                    high: Price::new(parse_string_f64(kline, "h")?).map_err(str::to_owned)?,
                    low: Price::new(parse_string_f64(kline, "l")?).map_err(str::to_owned)?,
                    close: Price::new(parse_string_f64(kline, "c")?).map_err(str::to_owned)?,
                    volume: Volume::new(parse_string_f64(kline, "v")?).map_err(str::to_owned)?,
                    trade_count: parse_string_i64(kline, "n")? as u64,
                    is_final: parse_string_bool(kline, "x")?,
                };

                return Ok(BinanceStreamEvent::Kline(candle));
            }

            if stream_name.ends_with("@miniTicker") {
                let open_price = parse_string_f64(data, "o")?;
                let last_price = parse_string_f64(data, "c")?;
                let price_change_24h = if open_price == 0.0 {
                    0.0
                } else {
                    ((last_price - open_price) / open_price) * 100.0
                };

                return Ok(BinanceStreamEvent::MiniTicker(LivePriceSnapshot {
                    instrument_id: instrument.id.clone(),
                    source_id: "binance".to_owned(),
                    last_price: Price::new(last_price).map_err(str::to_owned)?,
                    price_change_24h,
                    volume_24h: Volume::new(parse_string_f64(data, "v")?).map_err(str::to_owned)?,
                    observed_at: Timestamp::new(parse_string_i64(data, "E")?).map_err(str::to_owned)?,
                }));
            }

            Err(format!("unsupported combined stream payload: {stream_name}"))
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct KrakenMarketDataAdapter;

    impl KrakenMarketDataAdapter {
        pub fn rest_client(self) -> RestMarketDataClient {
            RestMarketDataClient::new("https://api.kraken.com")
        }

        pub fn ticker_path(self) -> &'static str {
            "/0/public/Ticker"
        }

        pub fn ohlc_path(self) -> &'static str {
            "/0/public/OHLC"
        }

        pub fn ticker_channel(self) -> &'static str {
            "ticker"
        }

        pub fn build_reference_price_request(self, pair: &str) -> RestRequest {
            RestRequest {
                path: self.ticker_path(),
                query: vec![("pair".to_owned(), pair.to_owned())],
            }
        }

        pub fn parse_reference_price_response(self, payload: &str) -> Result<Price, String> {
            let document = parse_json(payload)?;
            let errors = document
                .get("error")
                .and_then(Value::as_array)
                .ok_or_else(|| "missing Kraken error field".to_owned())?;
            if !errors.is_empty() {
                return Err(format!("Kraken ticker request returned errors: {errors:?}"));
            }

            let result = document
                .get("result")
                .and_then(Value::as_object)
                .ok_or_else(|| "missing Kraken ticker result".to_owned())?;
            let ticker = result
                .values()
                .next()
                .ok_or_else(|| "Kraken ticker result is empty".to_owned())?;
            let last_trade = ticker
                .get("c")
                .and_then(Value::as_array)
                .and_then(|values| values.first())
                .and_then(Value::as_str)
                .ok_or_else(|| "missing Kraken last trade price".to_owned())?
                .parse::<f64>()
                .map_err(|error| format!("invalid Kraken last trade price: {error}"))?;

            Price::new(last_trade).map_err(str::to_owned)
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

    impl SourceStatus {
        pub fn as_str(self) -> &'static str {
            match self {
                Self::Healthy => "healthy",
                Self::Degraded => "degraded",
                Self::Unavailable => "unavailable",
            }
        }
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

fn parse_string_bool(value: &Value, field: &str) -> Result<bool, String> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("missing boolean field: {field}"))
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
    use super::adapters::{BinanceMarketDataAdapter, BinanceStreamEvent, KrakenMarketDataAdapter};
    use super::clients::{StreamSubscription, WebSocketMarketDataClient};
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
        let rest_client = adapter.rest_client();
        let request = adapter.build_klines_request("BTCUSDT", Timeframe::OneMinute, 1000, None, None);
        let latest_price_request = adapter.build_latest_price_request("BTCUSDT");
        let market_summary_request = adapter.build_market_summary_request("BTCUSDT");

        assert_eq!(request.path, "/api/v3/klines");
        assert_eq!(request.query_value("symbol"), Some("BTCUSDT"));
        assert_eq!(request.query_value("interval"), Some("1m"));
        assert_eq!(request.query_value("limit"), Some("1000"));
        assert_eq!(
            rest_client.build_url(&request),
            "https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1m&limit=1000"
        );
        assert_eq!(latest_price_request.path, "/api/v3/ticker/price");
        assert_eq!(market_summary_request.path, "/api/v3/ticker/24hr");
    }

    #[test]
    fn builds_required_binance_stream_subscriptions() {
        let adapter = BinanceMarketDataAdapter;
        let subscriptions = adapter.build_primary_stream_subscriptions("BTCUSDT");
        let websocket_client = adapter.websocket_client();

        assert_eq!(subscriptions.len(), 2);
        assert_eq!(subscriptions[0].stream_name, "btcusdt@kline_1m");
        assert_eq!(subscriptions[1].stream_name, "btcusdt@miniTicker");
        assert_eq!(
            websocket_client
                .build_combined_stream_url(&subscriptions)
                .expect("combined stream url should build"),
            "wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/btcusdt@miniTicker"
        );
    }

    #[test]
    fn builds_subscribe_message_for_streams() {
        let websocket_client = WebSocketMarketDataClient::new("wss://stream.binance.com:9443");
        let message = websocket_client
            .build_subscribe_message(
                &[
                    StreamSubscription {
                        stream_name: "btcusdt@kline_1m".to_owned(),
                    },
                    StreamSubscription {
                        stream_name: "btcusdt@miniTicker".to_owned(),
                    },
                ],
                7,
            )
            .expect("subscribe message should build");

        assert_eq!(
            message,
            r#"{"id":7,"method":"SUBSCRIBE","params":["btcusdt@kline_1m","btcusdt@miniTicker"]}"#
        );
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
    fn builds_kraken_reference_price_request() {
        let adapter = KrakenMarketDataAdapter;
        let request = adapter.build_reference_price_request("XBTUSD");

        assert_eq!(request.path, "/0/public/Ticker");
        assert_eq!(request.query_value("pair"), Some("XBTUSD"));
        assert_eq!(
            adapter.rest_client().build_url(&request),
            "https://api.kraken.com/0/public/Ticker?pair=XBTUSD"
        );
    }

    #[test]
    fn parses_kraken_reference_price_response() {
        let adapter = KrakenMarketDataAdapter;
        let price = adapter
            .parse_reference_price_response(
                r#"{
                    "error":[],
                    "result":{
                        "XXBTZUSD":{
                            "a":["68460.1","1","1.000"],
                            "b":["68450.1","1","1.000"],
                            "c":["68455.12","0.001"],
                            "v":["100.0","200.0"]
                        }
                    }
                }"#,
            )
            .expect("Kraken ticker response should parse");

        assert_eq!(price.0, 68_455.12);
    }

    #[test]
    fn rejects_kraken_reference_price_errors() {
        let adapter = KrakenMarketDataAdapter;
        let error = adapter
            .parse_reference_price_response(
                r#"{"error":["EQuery:Unknown asset pair"],"result":{}}"#,
            )
            .expect_err("Kraken ticker errors should be surfaced");

        assert!(error.contains("Unknown asset pair"));
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
        assert_eq!(candle.source_id, "binance");
        assert_eq!(candle.timeframe, Timeframe::OneMinute);
        assert_eq!(candle.open.0, 68_000.0);
        assert_eq!(candle.close.0, 68_450.12);
        assert_eq!(candle.trade_count, 42);
        assert!(candle.is_final);
    }

    #[test]
    fn parses_klines_response_into_candles() {
        let adapter = BinanceMarketDataAdapter;
        let candles = adapter
            .parse_klines_response(
                &Instrument::btc_usd_spot(),
                Timeframe::OneMinute,
                r#"[
                    [1710000000000,"68000.00","68500.00","67950.00","68450.12","123.45",1710000059999,"0",42,"0","0","0"],
                    [1710000060000,"68450.12","68600.00","68400.00","68500.00","95.00",1710000119999,"0",31,"0","0","0"]
                ]"#,
            )
            .expect("kline response should parse");

        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].open.0, 68_000.0);
        assert_eq!(candles[1].close.0, 68_500.0);
    }

    #[test]
    fn parses_combined_kline_stream_payload() {
        let adapter = BinanceMarketDataAdapter;
        let event = adapter
            .parse_combined_stream_payload(
                &Instrument::btc_usd_spot(),
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
            )
            .expect("combined kline payload should parse");

        match event {
            BinanceStreamEvent::Kline(candle) => {
                assert_eq!(candle.timeframe, Timeframe::OneMinute);
                assert_eq!(candle.close.0, 68_500.0);
                assert!(!candle.is_final);
            }
            _ => panic!("expected kline event"),
        }
    }

    #[test]
    fn parses_combined_mini_ticker_stream_payload() {
        let adapter = BinanceMarketDataAdapter;
        let event = adapter
            .parse_combined_stream_payload(
                &Instrument::btc_usd_spot(),
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
            )
            .expect("combined mini ticker payload should parse");

        match event {
            BinanceStreamEvent::MiniTicker(snapshot) => {
                assert_eq!(snapshot.last_price.0, 68_500.0);
                assert!(snapshot.price_change_24h > 0.7);
                assert_eq!(snapshot.observed_at.0, 1_710_000_120_000);
            }
            _ => panic!("expected mini ticker event"),
        }
    }

    #[test]
    fn marks_stale_source_as_degraded() {
        let monitor = SourceHealthMonitor;
        let status = monitor.classify(10_000, 5_000, 2_000);
        assert_eq!(status, SourceStatus::Degraded);
    }
}
