use market_data_core::candle::Candle;
use market_data_core::timeframe::Timeframe;
use market_data_core::value_objects::Timestamp;

pub mod aggregation {
    use super::Candle;
    use market_data_core::timeframe::Timeframe;

    #[derive(Debug, Default, Clone, Copy)]
    pub struct CandleAggregationStrategy;

    impl CandleAggregationStrategy {
        pub fn finalize_active_candle(self, candle: Candle) -> Candle {
            candle.finalize()
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct TimeframeDerivationEngine;

    impl TimeframeDerivationEngine {
        pub fn default_targets(self) -> [Timeframe; 5] {
            [
                Timeframe::FiveMinutes,
                Timeframe::FifteenMinutes,
                Timeframe::OneHour,
                Timeframe::FourHours,
                Timeframe::OneDay,
            ]
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct CandleGapDetector;

    impl CandleGapDetector {
        pub fn has_gap(self, previous_close_ms: i64, next_open_ms: i64, timeframe: Timeframe) -> bool {
            next_open_ms.saturating_sub(previous_close_ms) > timeframe.minutes() * 60_000
        }
    }
}

pub mod features {
    use super::{Candle, Timeframe, Timestamp};

    #[derive(Debug, Clone, PartialEq)]
    pub struct FeatureSnapshot {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub observed_at: Timestamp,
        pub trend_score: f64,
        pub momentum_score: f64,
        pub volatility_score: f64,
        pub volume_confirmation_score: f64,
        pub support_distance: f64,
        pub resistance_distance: f64,
        pub level_reaction_score: f64,
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct TrendFeatureStrategy;

    impl TrendFeatureStrategy {
        pub fn compute(self, candles: &[Candle]) -> f64 {
            match (candles.first(), candles.last()) {
                (Some(first), Some(last)) => last.close.0 - first.open.0,
                _ => 0.0,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct MomentumFeatureStrategy;

    impl MomentumFeatureStrategy {
        pub fn compute(self, candles: &[Candle]) -> f64 {
            if candles.len() < 2 {
                return 0.0;
            }

            let last = candles[candles.len() - 1].close.0;
            let previous = candles[candles.len() - 2].close.0;
            last - previous
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct VolatilityFeatureStrategy;

    impl VolatilityFeatureStrategy {
        pub fn compute(self, candles: &[Candle]) -> f64 {
            if candles.is_empty() {
                return 0.0;
            }

            let sum = candles
                .iter()
                .map(|candle| candle.high.0 - candle.low.0)
                .sum::<f64>();

            sum / candles.len() as f64
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct LevelDetectionStrategy;

    impl LevelDetectionStrategy {
        pub fn detect(self, candles: &[Candle]) -> (f64, f64) {
            if candles.is_empty() {
                return (0.0, 0.0);
            }

            let support = candles
                .iter()
                .map(|candle| candle.low.0)
                .fold(f64::INFINITY, f64::min);
            let resistance = candles
                .iter()
                .map(|candle| candle.high.0)
                .fold(f64::NEG_INFINITY, f64::max);

            (support, resistance)
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct FeatureSnapshotBuilder;

    impl FeatureSnapshotBuilder {
        pub fn build(self, candles: &[Candle]) -> Option<FeatureSnapshot> {
            let first = candles.first()?;
            let last = candles.last()?;
            let trend_strategy = TrendFeatureStrategy;
            let momentum_strategy = MomentumFeatureStrategy;
            let volatility_strategy = VolatilityFeatureStrategy;
            let level_strategy = LevelDetectionStrategy;
            let (support, resistance) = level_strategy.detect(candles);

            Some(FeatureSnapshot {
                instrument_id: last.instrument_id.clone(),
                timeframe: last.timeframe,
                observed_at: last.close_time,
                trend_score: trend_strategy.compute(candles),
                momentum_score: momentum_strategy.compute(candles),
                volatility_score: volatility_strategy.compute(candles),
                volume_confirmation_score: last.volume.0 - first.volume.0,
                support_distance: (last.close.0 - support).max(0.0),
                resistance_distance: (resistance - last.close.0).max(0.0),
                level_reaction_score: resistance - support,
            })
        }
    }
}

pub mod regime {
    use super::features::FeatureSnapshot;
    use super::{Timeframe, Timestamp};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MarketRegimeLabel {
        Uptrend,
        Downtrend,
        Range,
        HighVolatilityTransition,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MarketRegimeSnapshot {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub observed_at: Timestamp,
        pub regime_label: MarketRegimeLabel,
        pub regime_score: f64,
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct MarketRegimeStrategy;

    impl MarketRegimeStrategy {
        pub fn classify(self, snapshot: &FeatureSnapshot) -> MarketRegimeSnapshot {
            let regime_label = if snapshot.volatility_score > 500.0 {
                MarketRegimeLabel::HighVolatilityTransition
            } else if snapshot.trend_score > 0.0 {
                MarketRegimeLabel::Uptrend
            } else if snapshot.trend_score < 0.0 {
                MarketRegimeLabel::Downtrend
            } else {
                MarketRegimeLabel::Range
            };

            MarketRegimeSnapshot {
                instrument_id: snapshot.instrument_id.clone(),
                timeframe: snapshot.timeframe,
                observed_at: snapshot.observed_at,
                regime_label,
                regime_score: snapshot.trend_score.abs() + snapshot.volatility_score,
            }
        }
    }
}

pub mod scenario {
    use super::features::FeatureSnapshot;
    use super::regime::{MarketRegimeLabel, MarketRegimeSnapshot};
    use super::{Timeframe, Timestamp};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ExpectedDirection {
        Bullish,
        Neutral,
        Bearish,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ScenarioSnapshot {
        pub instrument_id: String,
        pub timeframe: Timeframe,
        pub observed_at: Timestamp,
        pub bull_probability: f64,
        pub base_probability: f64,
        pub bear_probability: f64,
        pub trigger_level: f64,
        pub invalidation_level: f64,
        pub expected_direction: ExpectedDirection,
        pub explanation: String,
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct ScenarioScoringStrategy;

    impl ScenarioScoringStrategy {
        pub fn score(
            self,
            feature_snapshot: &FeatureSnapshot,
            regime_snapshot: &MarketRegimeSnapshot,
            explanation: String,
        ) -> ScenarioSnapshot {
            let bull_probability: f64 = match regime_snapshot.regime_label {
                MarketRegimeLabel::Uptrend => 0.55,
                MarketRegimeLabel::Range => 0.30,
                MarketRegimeLabel::Downtrend => 0.15,
                MarketRegimeLabel::HighVolatilityTransition => 0.34,
            };
            let bear_probability: f64 = match regime_snapshot.regime_label {
                MarketRegimeLabel::Uptrend => 0.15,
                MarketRegimeLabel::Range => 0.25,
                MarketRegimeLabel::Downtrend => 0.55,
                MarketRegimeLabel::HighVolatilityTransition => 0.33,
            };
            let base_probability = (1.0_f64 - bull_probability - bear_probability).max(0.0_f64);
            let expected_direction = if bull_probability > bear_probability {
                ExpectedDirection::Bullish
            } else if bear_probability > bull_probability {
                ExpectedDirection::Bearish
            } else {
                ExpectedDirection::Neutral
            };

            ScenarioSnapshot {
                instrument_id: feature_snapshot.instrument_id.clone(),
                timeframe: feature_snapshot.timeframe,
                observed_at: feature_snapshot.observed_at,
                bull_probability,
                base_probability,
                bear_probability,
                trigger_level: feature_snapshot.resistance_distance,
                invalidation_level: feature_snapshot.support_distance,
                expected_direction,
                explanation,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct ScenarioSnapshotFactory;

    impl ScenarioSnapshotFactory {
        pub fn build(
            self,
            feature_snapshot: &FeatureSnapshot,
            regime_snapshot: &MarketRegimeSnapshot,
            explanation: String,
        ) -> ScenarioSnapshot {
            ScenarioScoringStrategy.score(feature_snapshot, regime_snapshot, explanation)
        }
    }
}

pub mod explanations {
    use super::features::FeatureSnapshot;
    use super::regime::{MarketRegimeLabel, MarketRegimeSnapshot};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct ScenarioExplanationBuilder;

    impl ScenarioExplanationBuilder {
        pub fn build(self, feature_snapshot: &FeatureSnapshot, regime_snapshot: &MarketRegimeSnapshot) -> String {
            let regime = match regime_snapshot.regime_label {
                MarketRegimeLabel::Uptrend => "uptrend",
                MarketRegimeLabel::Downtrend => "downtrend",
                MarketRegimeLabel::Range => "range",
                MarketRegimeLabel::HighVolatilityTransition => "high-volatility transition",
            };

            format!(
                "BTC is in a {} with trend score {:.2}, momentum {:.2}, and volatility {:.2}.",
                regime, feature_snapshot.trend_score, feature_snapshot.momentum_score, feature_snapshot.volatility_score
            )
        }
    }
}

pub use explanations::ScenarioExplanationBuilder;
pub use features::{FeatureSnapshot, FeatureSnapshotBuilder, LevelDetectionStrategy, MomentumFeatureStrategy, TrendFeatureStrategy, VolatilityFeatureStrategy};
pub use regime::{MarketRegimeLabel, MarketRegimeSnapshot, MarketRegimeStrategy};
pub use scenario::{ExpectedDirection, ScenarioScoringStrategy, ScenarioSnapshot, ScenarioSnapshotFactory};

#[cfg(test)]
mod tests {
    use super::features::FeatureSnapshotBuilder;
    use super::{MarketRegimeStrategy, ScenarioExplanationBuilder, ScenarioScoringStrategy};
    use market_data_core::candle::Candle;
    use market_data_core::timeframe::Timeframe;
    use market_data_core::value_objects::{Price, Timestamp, Volume};

    fn build_candle(open: f64, high: f64, low: f64, close: f64, close_time: i64) -> Candle {
        Candle {
            instrument_id: "BTC-USD-SPOT".to_owned(),
            source_id: "binance".to_owned(),
            timeframe: Timeframe::OneMinute,
            open_time: Timestamp::new(close_time - 60_000).unwrap(),
            close_time: Timestamp::new(close_time).unwrap(),
            open: Price::new(open).unwrap(),
            high: Price::new(high).unwrap(),
            low: Price::new(low).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Volume::new(100.0).unwrap(),
            trade_count: 10,
            is_final: true,
        }
    }

    #[test]
    fn builds_probabilities_that_sum_to_one() {
        let candles = vec![
            build_candle(100_000.0, 101_000.0, 99_500.0, 100_500.0, 60_000),
            build_candle(100_500.0, 102_000.0, 100_000.0, 101_500.0, 120_000),
        ];
        let feature_snapshot = FeatureSnapshotBuilder.build(&candles).unwrap();
        let regime_snapshot = MarketRegimeStrategy.classify(&feature_snapshot);
        let explanation = ScenarioExplanationBuilder.build(&feature_snapshot, &regime_snapshot);
        let scenario = ScenarioScoringStrategy.score(&feature_snapshot, &regime_snapshot, explanation);

        let probability_sum = scenario.bull_probability + scenario.base_probability + scenario.bear_probability;
        assert!((probability_sum - 1.0).abs() < f64::EPSILON);
    }
}
