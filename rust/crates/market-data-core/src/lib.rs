pub mod instrument {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MarketType {
        Spot,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Instrument {
        pub id: String,
        pub symbol: String,
        pub base_asset: String,
        pub quote_asset: String,
        pub market_type: MarketType,
    }

    impl Instrument {
        pub fn btc_usd_spot() -> Self {
            Self {
                id: "BTC-USD-SPOT".to_owned(),
                symbol: "BTC-USD-SPOT".to_owned(),
                base_asset: "BTC".to_owned(),
                quote_asset: "USD".to_owned(),
                market_type: MarketType::Spot,
            }
        }
    }
}

pub mod source {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SourceType {
        Exchange,
        Reference,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DataSource {
        pub id: String,
        pub name: String,
        pub source_type: SourceType,
    }

    impl DataSource {
        pub fn binance() -> Self {
            Self {
                id: "binance".to_owned(),
                name: "Binance".to_owned(),
                source_type: SourceType::Exchange,
            }
        }

        pub fn kraken() -> Self {
            Self {
                id: "kraken".to_owned(),
                name: "Kraken".to_owned(),
                source_type: SourceType::Exchange,
            }
        }

        pub fn coingecko() -> Self {
            Self {
                id: "coingecko".to_owned(),
                name: "CoinGecko".to_owned(),
                source_type: SourceType::Reference,
            }
        }
    }
}

pub mod timeframe {
    use core::fmt;
    use core::str::FromStr;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Timeframe {
        OneMinute,
        FiveMinutes,
        FifteenMinutes,
        OneHour,
        FourHours,
        OneDay,
        OneWeek,
    }

    impl Timeframe {
        pub fn as_str(self) -> &'static str {
            match self {
                Self::OneMinute => "1m",
                Self::FiveMinutes => "5m",
                Self::FifteenMinutes => "15m",
                Self::OneHour => "1h",
                Self::FourHours => "4h",
                Self::OneDay => "1d",
                Self::OneWeek => "1w",
            }
        }

        pub fn minutes(self) -> i64 {
            match self {
                Self::OneMinute => 1,
                Self::FiveMinutes => 5,
                Self::FifteenMinutes => 15,
                Self::OneHour => 60,
                Self::FourHours => 240,
                Self::OneDay => 1_440,
                Self::OneWeek => 10_080,
            }
        }
    }

    impl fmt::Display for Timeframe {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.as_str())
        }
    }

    impl FromStr for Timeframe {
        type Err = &'static str;

        fn from_str(value: &str) -> Result<Self, Self::Err> {
            match value {
                "1m" => Ok(Self::OneMinute),
                "5m" => Ok(Self::FiveMinutes),
                "15m" => Ok(Self::FifteenMinutes),
                "1h" => Ok(Self::OneHour),
                "4h" => Ok(Self::FourHours),
                "1d" => Ok(Self::OneDay),
                "1w" => Ok(Self::OneWeek),
                _ => Err("unsupported timeframe"),
            }
        }
    }
}

pub mod value_objects {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Price(pub f64);

    impl Price {
        pub fn new(value: f64) -> Result<Self, &'static str> {
            if value.is_finite() && value >= 0.0 {
                return Ok(Self(value));
            }

            Err("price must be finite and non-negative")
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Volume(pub f64);

    impl Volume {
        pub fn new(value: f64) -> Result<Self, &'static str> {
            if value.is_finite() && value >= 0.0 {
                return Ok(Self(value));
            }

            Err("volume must be finite and non-negative")
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Timestamp(pub i64);

    impl Timestamp {
        pub fn new(value: i64) -> Result<Self, &'static str> {
            if value >= 0 {
                return Ok(Self(value));
            }

            Err("timestamp must be non-negative")
        }
    }
}

pub mod candle {
    use crate::timeframe::Timeframe;
    use crate::value_objects::{Price, Timestamp, Volume};

    #[derive(Debug, Clone, PartialEq)]
    pub struct Candle {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub open_time: Timestamp,
        pub close_time: Timestamp,
        pub open: Price,
        pub high: Price,
        pub low: Price,
        pub close: Price,
        pub volume: Volume,
        pub trade_count: u64,
        pub is_final: bool,
    }

    impl Candle {
        pub fn finalize(mut self) -> Self {
            self.is_final = true;
            self
        }
    }
}

pub mod snapshot {
    use crate::value_objects::{Price, Timestamp, Volume};

    #[derive(Debug, Clone, PartialEq)]
    pub struct LivePriceSnapshot {
        pub instrument_id: String,
        pub source_id: String,
        pub last_price: Price,
        pub price_change_24h: f64,
        pub volume_24h: Volume,
        pub observed_at: Timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::instrument::Instrument;
    use super::timeframe::Timeframe;
    use core::str::FromStr;

    #[test]
    fn parses_supported_timeframe() {
        assert_eq!(Timeframe::from_str("4h"), Ok(Timeframe::FourHours));
        assert_eq!(Timeframe::FourHours.to_string(), "4h");
    }

    #[test]
    fn builds_btc_spot_instrument() {
        let instrument = Instrument::btc_usd_spot();
        assert_eq!(instrument.symbol, "BTC-USD-SPOT");
        assert_eq!(instrument.base_asset, "BTC");
    }
}
