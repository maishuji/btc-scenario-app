PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS instruments (
    id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE,
    base_asset TEXT NOT NULL,
    quote_asset TEXT NOT NULL,
    market_type TEXT NOT NULL CHECK (market_type IN ('spot')),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS data_sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    source_type TEXT NOT NULL CHECK (source_type IN ('exchange', 'reference')),
    is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0, 1)),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS candles (
    id TEXT PRIMARY KEY,
    instrument_id TEXT NOT NULL REFERENCES instruments(id),
    source_id TEXT NOT NULL REFERENCES data_sources(id),
    timeframe TEXT NOT NULL,
    open_time_ms INTEGER NOT NULL,
    close_time_ms INTEGER NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    trade_count INTEGER NOT NULL DEFAULT 0,
    is_final INTEGER NOT NULL DEFAULT 0 CHECK (is_final IN (0, 1)),
    created_at_ms INTEGER NOT NULL,
    CHECK (high >= low),
    CHECK (open >= 0.0),
    CHECK (high >= 0.0),
    CHECK (low >= 0.0),
    CHECK (close >= 0.0),
    CHECK (volume >= 0.0),
    UNIQUE (instrument_id, source_id, timeframe, open_time_ms)
);

CREATE TABLE IF NOT EXISTS live_price_snapshots (
    id TEXT PRIMARY KEY,
    instrument_id TEXT NOT NULL REFERENCES instruments(id),
    source_id TEXT NOT NULL REFERENCES data_sources(id),
    last_price REAL NOT NULL,
    price_change_24h REAL NOT NULL,
    volume_24h REAL NOT NULL,
    observed_at_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    CHECK (last_price >= 0.0),
    CHECK (volume_24h >= 0.0)
);

CREATE TABLE IF NOT EXISTS feature_snapshots (
    id TEXT PRIMARY KEY,
    instrument_id TEXT NOT NULL REFERENCES instruments(id),
    timeframe TEXT NOT NULL,
    observed_at_ms INTEGER NOT NULL,
    trend_score REAL NOT NULL,
    momentum_score REAL NOT NULL,
    volatility_score REAL NOT NULL,
    volume_confirmation_score REAL NOT NULL,
    support_level REAL NOT NULL DEFAULT 0.0,
    resistance_level REAL NOT NULL DEFAULT 0.0,
    support_distance REAL NOT NULL,
    resistance_distance REAL NOT NULL,
    level_reaction_score REAL NOT NULL,
    feature_version TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    CHECK (support_distance >= 0.0),
    CHECK (resistance_distance >= 0.0),
    UNIQUE (instrument_id, timeframe, observed_at_ms, feature_version)
);

CREATE TABLE IF NOT EXISTS regime_snapshots (
    id TEXT PRIMARY KEY,
    instrument_id TEXT NOT NULL REFERENCES instruments(id),
    timeframe TEXT NOT NULL,
    observed_at_ms INTEGER NOT NULL,
    regime_label TEXT NOT NULL CHECK (regime_label IN ('uptrend', 'downtrend', 'range', 'high_volatility_transition')),
    regime_score REAL NOT NULL,
    regime_version TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    UNIQUE (instrument_id, timeframe, observed_at_ms, regime_version)
);

CREATE TABLE IF NOT EXISTS scenario_snapshots (
    id TEXT PRIMARY KEY,
    instrument_id TEXT NOT NULL REFERENCES instruments(id),
    timeframe TEXT NOT NULL,
    observed_at_ms INTEGER NOT NULL,
    bull_probability REAL NOT NULL,
    base_probability REAL NOT NULL,
    bear_probability REAL NOT NULL,
    trigger_level REAL NOT NULL,
    invalidation_level REAL NOT NULL,
    expected_direction TEXT NOT NULL CHECK (expected_direction IN ('bullish', 'neutral', 'bearish')),
    explanation TEXT NOT NULL,
    scenario_version TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    CHECK (bull_probability >= 0.0 AND bull_probability <= 1.0),
    CHECK (base_probability >= 0.0 AND base_probability <= 1.0),
    CHECK (bear_probability >= 0.0 AND bear_probability <= 1.0),
    CHECK (ABS((bull_probability + base_probability + bear_probability) - 1.0) <= 0.000001),
    UNIQUE (instrument_id, timeframe, observed_at_ms, scenario_version)
);

CREATE TABLE IF NOT EXISTS alerts (
    id TEXT PRIMARY KEY,
    instrument_id TEXT NOT NULL REFERENCES instruments(id),
    timeframe TEXT NOT NULL,
    alert_type TEXT NOT NULL CHECK (alert_type IN ('regime_changed', 'trigger_crossed', 'invalidation_crossed', 'scenario_shifted')),
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    message TEXT NOT NULL,
    triggered_at_ms INTEGER NOT NULL,
    scenario_snapshot_id TEXT REFERENCES scenario_snapshots(id),
    is_acknowledged INTEGER NOT NULL DEFAULT 0 CHECK (is_acknowledged IN (0, 1)),
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS source_health_events (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES data_sources(id),
    status TEXT NOT NULL CHECK (status IN ('healthy', 'degraded', 'unavailable')),
    message TEXT NOT NULL,
    observed_at_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_candles_instrument_timeframe_open_time
    ON candles (instrument_id, timeframe, open_time_ms DESC);

CREATE INDEX IF NOT EXISTS idx_candles_source_timeframe_open_time
    ON candles (source_id, timeframe, open_time_ms DESC);

CREATE INDEX IF NOT EXISTS idx_live_price_snapshots_instrument_observed_at
    ON live_price_snapshots (instrument_id, observed_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_feature_snapshots_instrument_timeframe_observed_at
    ON feature_snapshots (instrument_id, timeframe, observed_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_regime_snapshots_instrument_timeframe_observed_at
    ON regime_snapshots (instrument_id, timeframe, observed_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_scenario_snapshots_instrument_timeframe_observed_at
    ON scenario_snapshots (instrument_id, timeframe, observed_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_alerts_instrument_triggered_at
    ON alerts (instrument_id, triggered_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_source_health_events_source_observed_at
    ON source_health_events (source_id, observed_at_ms DESC);

INSERT INTO instruments (
    id,
    symbol,
    base_asset,
    quote_asset,
    market_type,
    is_active,
    created_at_ms,
    updated_at_ms
)
VALUES (
    'BTC-USD-SPOT',
    'BTC-USD-SPOT',
    'BTC',
    'USD',
    'spot',
    1,
    0,
    0
)
ON CONFLICT(id) DO NOTHING;

INSERT INTO data_sources (
    id,
    name,
    source_type,
    is_primary,
    is_active,
    created_at_ms,
    updated_at_ms
)
VALUES
    ('binance', 'Binance', 'exchange', 1, 1, 0, 0),
    ('kraken', 'Kraken', 'exchange', 0, 1, 0, 0),
    ('coingecko', 'CoinGecko', 'reference', 0, 1, 0, 0)
ON CONFLICT(id) DO NOTHING;
