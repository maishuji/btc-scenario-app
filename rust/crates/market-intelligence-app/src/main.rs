mod config {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AppConfig {
        pub instrument_symbol: String,
        pub primary_exchange_symbol: String,
    }

    impl Default for AppConfig {
        fn default() -> Self {
            Self {
                instrument_symbol: "BTC-USD-SPOT".to_owned(),
                primary_exchange_symbol: "BTCUSDT".to_owned(),
            }
        }
    }
}

mod tasks {
    use crate::config::AppConfig;
    use market_data_core::instrument::Instrument;
    use market_data_infrastructure::adapters::BinanceMarketDataAdapter;

    #[derive(Debug, Default, Clone, Copy)]
    pub struct StartupTaskRunner;

    impl StartupTaskRunner {
        pub fn describe(self, config: &AppConfig) -> String {
            let instrument = Instrument::btc_usd_spot();
            let adapter = BinanceMarketDataAdapter;
            let stream_name = adapter.kline_stream_name(&config.primary_exchange_symbol, "1m");

            format!(
                "starting {} with primary stream {} for {}",
                instrument.id, stream_name, config.instrument_symbol
            )
        }
    }
}

mod runtime {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Runtime {
        pub startup_summary: String,
    }

    impl Runtime {
        pub fn run(&self) -> &str {
            &self.startup_summary
        }
    }
}

mod bootstrap {
    use crate::config::AppConfig;
    use crate::runtime::Runtime;
    use crate::tasks::StartupTaskRunner;

    #[derive(Debug, Default, Clone, Copy)]
    pub struct Bootstrap;

    impl Bootstrap {
        pub fn build(self, config: &AppConfig) -> Runtime {
            let startup_summary = StartupTaskRunner.describe(config);
            Runtime { startup_summary }
        }
    }
}

fn main() {
    let config = config::AppConfig::default();
    let runtime = bootstrap::Bootstrap.build(&config);
    println!("{}", runtime.run());
}
