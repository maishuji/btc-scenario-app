import { startTransition, useEffect, useState } from 'react'
import './App.css'

type MarketOverview = {
  instrument_id: string
  timeframe: string
  observed_at_ms: number
  last_price: number
  price_change_24h: number
  volume_24h: number
  trend_score: number
  momentum_score: number
  volatility_score: number
  volume_confirmation_score: number
  support_level: number
  resistance_level: number
  support_distance: number
  resistance_distance: number
  level_reaction_score: number
  regime_label: string
  regime_score: number
  bull_probability: number
  base_probability: number
  bear_probability: number
  trigger_level: number
  invalidation_level: number
  expected_direction: string
  explanation: string
}

type Candle = {
  instrument_id: string
  source_id: string
  timeframe: string
  open_time_ms: number
  close_time_ms: number
  open: number
  high: number
  low: number
  close: number
  volume: number
  trade_count: number
  is_final: boolean
}

type ScenarioHistoryEntry = {
  instrument_id: string
  timeframe: string
  observed_at_ms: number
  bull_probability: number
  base_probability: number
  bear_probability: number
  trigger_level: number
  invalidation_level: number
  expected_direction: string
  explanation: string
}

type AlertEntry = {
  instrument_id: string
  timeframe: string
  alert_type: string
  severity: string
  message: string
  triggered_at_ms: number
  is_acknowledged: boolean
}

type SourceHealth = {
  source_id: string
  status: string
  message: string
  observed_at_ms: number
  last_successful_update_ms: number | null
  age_ms: number | null
}

type DashboardSnapshot = {
  overview: MarketOverview
  candles: Candle[]
  scenarios: ScenarioHistoryEntry[]
  alerts: AlertEntry[]
  source_health: SourceHealth[]
}

type ProjectedScenario = {
  observed_at_ms: number
  trigger_level: number
  invalidation_level: number
  expected_direction: string
  explanation: string
}

const pollIntervalMs = 15_000
const chartWidth = 720
const chartHeight = 260
const timeframeOptions = ['1m', '5m', '15m', '1h', '4h', '1d', '1w'] as const
type Timeframe = (typeof timeframeOptions)[number]

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
})

const compactNumberFormatter = new Intl.NumberFormat('en-US', {
  notation: 'compact',
  maximumFractionDigits: 1,
})

const percentFormatter = new Intl.NumberFormat('en-US', {
  style: 'percent',
  maximumFractionDigits: 0,
})

const clockFormatter = new Intl.DateTimeFormat('en-US', {
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
})

const shortDateFormatter = new Intl.DateTimeFormat('en-US', {
  month: 'short',
  day: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})

async function loadJson<T>(url: string): Promise<T> {
  const response = await fetch(url)

  if (!response.ok) {
    throw new Error(`Request failed for ${url}: ${response.status}`)
  }

  return response.json() as Promise<T>
}

function formatProbability(value: number) {
  return percentFormatter.format(value)
}

function formatCurrency(value: number) {
  return currencyFormatter.format(value)
}

function formatCompactNumber(value: number) {
  return compactNumberFormatter.format(value)
}

function formatSignedPercent(value: number) {
  const sign = value > 0 ? '+' : ''
  return `${sign}${value.toFixed(2)}%`
}

function formatTimestamp(value: number) {
  return shortDateFormatter.format(new Date(value))
}

function formatDistanceToLevel(level: number, reference: number) {
  const delta = level - reference
  const percent = reference === 0 ? 0 : (delta / reference) * 100
  const sign = delta > 0 ? '+' : ''

  return `${sign}${percent.toFixed(2)}%`
}

function chartExtents(
  candles: Candle[],
  projectedScenario: ProjectedScenario,
  spotPrice: number,
  supportLevel: number,
  resistanceLevel: number,
) {
  if (candles.length === 0) {
    return {
      low: Math.min(projectedScenario.invalidation_level, supportLevel, spotPrice),
      high: Math.max(projectedScenario.trigger_level, resistanceLevel, spotPrice),
    }
  }

  const values = candles.flatMap((candle) => [candle.low, candle.high])
  values.push(
    projectedScenario.trigger_level,
    projectedScenario.invalidation_level,
    supportLevel,
    resistanceLevel,
    spotPrice,
  )

  const low = Math.min(...values)
  const high = Math.max(...values)
  const padding = Math.max((high - low) * 0.08, 1)

  return {
    low: low - padding,
    high: high + padding,
  }
}

function scaleChartValue(value: number, extents: { low: number; high: number }) {
  const span = extents.high - extents.low || 1
  return chartHeight - ((value - extents.low) / span) * chartHeight
}

function chartPath(candles: Candle[], extents: { low: number; high: number }) {
  if (candles.length === 0) {
    return ''
  }

  return candles
    .map((candle, index) => {
      const x = candles.length === 1 ? chartWidth / 2 : (index / (candles.length - 1)) * chartWidth
      const y = scaleChartValue(candle.close, extents)
      return `${index === 0 ? 'M' : 'L'} ${x.toFixed(2)} ${y.toFixed(2)}`
    })
    .join(' ')
}

function chartBounds(candles: Candle[]) {
  if (candles.length === 0) {
    return { low: 0, high: 0, last: 0 }
  }

  const closes = candles.map((candle) => candle.close)
  const lastClose = closes[closes.length - 1] ?? closes[0]

  return {
    low: Math.min(...closes),
    high: Math.max(...closes),
    last: lastClose,
  }
}

function regimeTone(label: string) {
  switch (label) {
    case 'uptrend':
      return 'tone-positive'
    case 'downtrend':
      return 'tone-negative'
    case 'high_volatility_transition':
      return 'tone-warning'
    default:
      return 'tone-neutral'
  }
}

function directionTone(direction: string) {
  switch (direction) {
    case 'bullish':
      return 'tone-positive'
    case 'bearish':
      return 'tone-negative'
    default:
      return 'tone-neutral'
  }
}

function severityTone(severity: string) {
  switch (severity) {
    case 'critical':
      return 'tone-negative'
    case 'warning':
      return 'tone-warning'
    default:
      return 'tone-neutral'
  }
}

function healthTone(status: string) {
  switch (status) {
    case 'healthy':
      return 'tone-positive'
    case 'degraded':
      return 'tone-warning'
    default:
      return 'tone-negative'
  }
}

function formatAge(ageMs: number | null) {
  if (ageMs === null) {
    return 'unknown'
  }

  const ageSeconds = Math.max(0, Math.floor(ageMs / 1_000))
  if (ageSeconds < 60) {
    return `${ageSeconds}s ago`
  }

  const ageMinutes = Math.floor(ageSeconds / 60)
  if (ageMinutes < 60) {
    return `${ageMinutes}m ago`
  }

  return `${Math.floor(ageMinutes / 60)}h ago`
}

function hasMatchingScenario(
  scenarios: ScenarioHistoryEntry[],
  triggeredAtMs: number,
) {
  return scenarios.some((scenario) => scenario.observed_at_ms === triggeredAtMs)
}

function App() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshedAtMs, setRefreshedAtMs] = useState<number | null>(null)
  const [selectedScenarioObservedAtMs, setSelectedScenarioObservedAtMs] = useState<number | null>(null)
  const [selectedTimeframe, setSelectedTimeframe] = useState<Timeframe>('1m')

  useEffect(() => {
    let cancelled = false
    let isFirstLoad = true
    const timeframeQuery = new URLSearchParams({ timeframe: selectedTimeframe })

    const hydrate = async () => {
      if (isFirstLoad) {
        setIsLoading(true)
      }

      try {
        const [overview, candles, scenarios, alerts, source_health] = await Promise.all([
          loadJson<MarketOverview>(`/api/market-overview?${timeframeQuery}`),
          loadJson<Candle[]>(`/api/candles?${timeframeQuery}&limit=48`),
          loadJson<ScenarioHistoryEntry[]>(`/api/scenario-history?${timeframeQuery}&limit=6`),
          loadJson<AlertEntry[]>(`/api/alerts?${timeframeQuery}&limit=6`),
          loadJson<SourceHealth[]>('/api/source-health'),
        ])

        if (cancelled) {
          return
        }

        startTransition(() => {
          setSnapshot({ overview, candles, scenarios, alerts, source_health })
          setRefreshedAtMs(Date.now())
          setError(null)
          setIsLoading(false)
        })
      } catch (loadError) {
        if (cancelled) {
          return
        }

        setError(
          loadError instanceof Error
            ? loadError.message
            : 'Dashboard data could not be loaded.',
        )
        setIsLoading(false)
      }

      isFirstLoad = false
    }

    void hydrate()
    const intervalId = window.setInterval(() => {
      void hydrate()
    }, pollIntervalMs)

    return () => {
      cancelled = true
      window.clearInterval(intervalId)
    }
  }, [selectedTimeframe])

  useEffect(() => {
    setSelectedScenarioObservedAtMs(null)
  }, [selectedTimeframe])

  if (isLoading && !snapshot) {
    return (
      <main className="app-shell app-shell-loading">
        <div className="loading-panel">
          <p className="eyebrow">BTC Scenario Terminal</p>
          <h1>Preparing the market tape.</h1>
          <p>
            Polling the Rust API for overview, candles, scenarios, and alerts.
          </p>
        </div>
      </main>
    )
  }

  if (!snapshot) {
    return (
      <main className="app-shell app-shell-loading">
        <div className="loading-panel loading-panel-error">
          <p className="eyebrow">BTC Scenario Terminal</p>
          <h1>Dashboard unavailable.</h1>
          <p>{error ?? 'The API did not return a usable market snapshot.'}</p>
        </div>
      </main>
    )
  }

  const { overview, candles, scenarios, alerts, source_health } = snapshot
  const binanceHealth = source_health.find((source) => source.source_id === 'binance')
  const feedStatus = binanceHealth?.status ?? 'unavailable'
  const bounds = chartBounds(candles)
  const lastCandle = candles[candles.length - 1]
  const selectedScenario = scenarios.find(
    (scenario) => scenario.observed_at_ms === selectedScenarioObservedAtMs,
  )
  const latestScenario = scenarios[scenarios.length - 1]
  const projectedScenario: ProjectedScenario = selectedScenario ?? latestScenario ?? {
    observed_at_ms: overview.observed_at_ms,
    trigger_level: overview.trigger_level,
    invalidation_level: overview.invalidation_level,
    expected_direction: overview.expected_direction,
    explanation: overview.explanation,
  }
  const isHistoricalProjection = projectedScenario.observed_at_ms !== overview.observed_at_ms
  const extents = chartExtents(
    candles,
    projectedScenario,
    overview.last_price,
    overview.support_level,
    overview.resistance_level,
  )
  const supportLineY = scaleChartValue(overview.support_level, extents)
  const resistanceLineY = scaleChartValue(overview.resistance_level, extents)
  const triggerLineY = scaleChartValue(projectedScenario.trigger_level, extents)
  const invalidationLineY = scaleChartValue(projectedScenario.invalidation_level, extents)
  const lastPriceLineY = scaleChartValue(overview.last_price, extents)

  return (
    <main className="app-shell">
      <section className="hero-panel panel">
        <div className="hero-copy">
          <p className="eyebrow">BTC Scenario Terminal</p>
          <h1>Live regime, scenario, and alert posture across BTC timeframes.</h1>
          <p className="hero-summary">{overview.explanation}</p>
          <div className="hero-tags">
            <span className={`tag ${regimeTone(overview.regime_label)}`}>
              Regime {overview.regime_label.replace(/_/g, ' ')}
            </span>
            <span className={`tag ${directionTone(overview.expected_direction)}`}>
              Bias {overview.expected_direction}
            </span>
            <span className="tag tone-neutral">View {overview.timeframe}</span>
            <span
              className={`tag ${healthTone(feedStatus)}`}
              title={binanceHealth?.message ?? 'No Binance health event has been recorded'}
            >
              Feed {feedStatus}
            </span>
          </div>
          <div className="timeframe-control">
            <span className="metric-label">Analysis timeframe</span>
            <div className="timeframe-options" role="group" aria-label="Analysis timeframe">
              {timeframeOptions.map((timeframe) => (
                <button
                  key={timeframe}
                  type="button"
                  className={`timeframe-button${selectedTimeframe === timeframe ? ' timeframe-button-active' : ''}`}
                  onClick={() => setSelectedTimeframe(timeframe)}
                  aria-pressed={selectedTimeframe === timeframe}
                >
                  {timeframe}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="hero-price panel-inset">
          <div>
            <p className="metric-label">Spot price</p>
            <p className="metric-value">{formatCurrency(overview.last_price)}</p>
          </div>
          <div className="metric-row">
            <span className="metric-label">24h move</span>
            <span className={overview.price_change_24h >= 0 ? 'metric-positive' : 'metric-negative'}>
              {formatSignedPercent(overview.price_change_24h)}
            </span>
          </div>
          <div className="metric-row">
            <span className="metric-label">24h volume</span>
            <span>{formatCompactNumber(overview.volume_24h)}</span>
          </div>
          <div className="metric-row">
            <span className="metric-label">Refresh</span>
            <span>{refreshedAtMs ? clockFormatter.format(new Date(refreshedAtMs)) : 'waiting'}</span>
          </div>
          <div className="metric-row">
            <span className="metric-label">Feed age</span>
            <span className={healthTone(feedStatus)}>{formatAge(binanceHealth?.age_ms ?? null)}</span>
          </div>
        </div>
      </section>

      <section className="summary-grid">
        <article className="panel stat-panel">
          <p className="metric-label">Trend score</p>
          <p className="stat-value">{overview.trend_score.toFixed(1)}</p>
          <p className="stat-caption">Regime confidence {overview.regime_score.toFixed(1)}</p>
        </article>
        <article className="panel stat-panel">
          <p className="metric-label">Momentum score</p>
          <p className="stat-value">{overview.momentum_score.toFixed(1)}</p>
          <p className="stat-caption">Volatility {overview.volatility_score.toFixed(1)}</p>
        </article>
        <article className="panel stat-panel">
          <p className="metric-label">Trigger level</p>
          <p className="stat-value">{formatCurrency(overview.trigger_level)}</p>
          <p className="stat-caption">Invalidation {formatCurrency(overview.invalidation_level)}</p>
        </article>
        <article className="panel stat-panel">
          <p className="metric-label">Level reaction</p>
          <p className="stat-value">{overview.level_reaction_score.toFixed(1)}</p>
          <p className="stat-caption">Volume confirmation {overview.volume_confirmation_score.toFixed(1)}</p>
        </article>
      </section>

      <section className="content-grid">
        <article className="panel chart-panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Market structure</p>
              <h2>Recent candle close path</h2>
            </div>
            <div className="panel-header-meta">
              <span>{candles.length} candles</span>
              <span>{lastCandle ? formatTimestamp(lastCandle.close_time_ms) : 'No close yet'}</span>
            </div>
          </div>

          <div className="chart-copy">
            <div>
              <span className="metric-label">Range low</span>
              <strong>{formatCurrency(bounds.low)}</strong>
            </div>
            <div>
              <span className="metric-label">Range high</span>
              <strong>{formatCurrency(bounds.high)}</strong>
            </div>
            <div>
              <span className="metric-label">Last close</span>
              <strong>{formatCurrency(bounds.last)}</strong>
            </div>
            <div>
              <span className="metric-label">Trigger spread</span>
              <strong>{formatDistanceToLevel(projectedScenario.trigger_level, overview.last_price)}</strong>
            </div>
          </div>

          <div className="projection-banner">
            <span className={`tag ${directionTone(projectedScenario.expected_direction)}`}>
              {isHistoricalProjection ? 'Historical projection' : 'Live projection'}
            </span>
            <span>{formatTimestamp(projectedScenario.observed_at_ms)}</span>
          </div>

          <div className="level-legend">
            <div className="level-legend-item">
              <span className="level-swatch level-swatch-support" />
              <span>Support {formatCurrency(overview.support_level)}</span>
            </div>
            <div className="level-legend-item">
              <span className="level-swatch level-swatch-resistance" />
              <span>Resistance {formatCurrency(overview.resistance_level)}</span>
            </div>
            <div className="level-legend-item">
              <span className="level-swatch level-swatch-trigger" />
              <span>Trigger {formatCurrency(projectedScenario.trigger_level)}</span>
            </div>
            <div className="level-legend-item">
              <span className="level-swatch level-swatch-last" />
              <span>Spot {formatCurrency(overview.last_price)}</span>
            </div>
            <div className="level-legend-item">
              <span className="level-swatch level-swatch-invalidation" />
              <span>Invalidation {formatCurrency(projectedScenario.invalidation_level)}</span>
            </div>
          </div>

          <svg viewBox={`0 0 ${chartWidth} ${chartHeight}`} className="price-chart" role="img" aria-label="Recent BTC close price chart with scenario levels">
            <defs>
              <linearGradient id="priceGlow" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="rgba(255, 122, 24, 0.8)" />
                <stop offset="100%" stopColor="rgba(255, 122, 24, 0.05)" />
              </linearGradient>
            </defs>
            <line x1="0" y1={supportLineY} x2={chartWidth} y2={supportLineY} className="scenario-line scenario-line-support" />
            <line x1="0" y1={resistanceLineY} x2={chartWidth} y2={resistanceLineY} className="scenario-line scenario-line-resistance" />
            <line x1="0" y1={triggerLineY} x2={chartWidth} y2={triggerLineY} className="scenario-line scenario-line-trigger" />
            <line x1="0" y1={lastPriceLineY} x2={chartWidth} y2={lastPriceLineY} className="scenario-line scenario-line-last" />
            <line x1="0" y1={invalidationLineY} x2={chartWidth} y2={invalidationLineY} className="scenario-line scenario-line-invalidation" />
            <text x="14" y={Math.max(supportLineY - 8, 18)} className="scenario-label scenario-label-support">
              Support {formatCurrency(overview.support_level)}
            </text>
            <text x="14" y={Math.max(resistanceLineY - 8, 18)} className="scenario-label scenario-label-resistance">
              Resistance {formatCurrency(overview.resistance_level)}
            </text>
            <text x="14" y={Math.max(triggerLineY - 8, 18)} className="scenario-label scenario-label-trigger">
              Trigger {formatCurrency(projectedScenario.trigger_level)}
            </text>
            <text x="14" y={Math.max(lastPriceLineY - 8, 18)} className="scenario-label scenario-label-last">
              Spot {formatCurrency(overview.last_price)}
            </text>
            <text x="14" y={Math.max(invalidationLineY - 8, 18)} className="scenario-label scenario-label-invalidation">
              Invalidation {formatCurrency(projectedScenario.invalidation_level)}
            </text>
            <path d={chartPath(candles, extents)} className="price-line-shadow" />
            <path d={chartPath(candles, extents)} className="price-line" />
          </svg>

          <div className="level-summary-grid">
            <article className="level-summary-card">
              <span className="metric-label">Support level</span>
              <strong>{formatCurrency(overview.support_level)}</strong>
              <span>{formatDistanceToLevel(overview.support_level, overview.last_price)} from spot</span>
            </article>
            <article className="level-summary-card">
              <span className="metric-label">Resistance level</span>
              <strong>{formatCurrency(overview.resistance_level)}</strong>
              <span>{formatDistanceToLevel(overview.resistance_level, overview.last_price)} from spot</span>
            </article>
            <article className="level-summary-card">
              <span className="metric-label">Bias trigger</span>
              <strong>{formatCurrency(projectedScenario.trigger_level)}</strong>
              <span>{formatDistanceToLevel(projectedScenario.trigger_level, overview.last_price)} from spot</span>
            </article>
            <article className="level-summary-card">
              <span className="metric-label">Risk invalidation</span>
              <strong>{formatCurrency(projectedScenario.invalidation_level)}</strong>
              <span>{formatDistanceToLevel(projectedScenario.invalidation_level, overview.last_price)} from spot</span>
            </article>
          </div>
        </article>

        <article className="panel probability-panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Scenario balance</p>
              <h2>Probability stack</h2>
            </div>
            <span className={`tag ${directionTone(overview.expected_direction)}`}>
              {overview.expected_direction}
            </span>
          </div>

          <div className="probability-bars">
            <div>
              <div className="probability-label-row">
                <span>Bull</span>
                <span>{formatProbability(overview.bull_probability)}</span>
              </div>
              <div className="bar-track"><div className="bar-fill bar-fill-bull" style={{ width: `${overview.bull_probability * 100}%` }} /></div>
            </div>
            <div>
              <div className="probability-label-row">
                <span>Base</span>
                <span>{formatProbability(overview.base_probability)}</span>
              </div>
              <div className="bar-track"><div className="bar-fill bar-fill-base" style={{ width: `${overview.base_probability * 100}%` }} /></div>
            </div>
            <div>
              <div className="probability-label-row">
                <span>Bear</span>
                <span>{formatProbability(overview.bear_probability)}</span>
              </div>
              <div className="bar-track"><div className="bar-fill bar-fill-bear" style={{ width: `${overview.bear_probability * 100}%` }} /></div>
            </div>
          </div>

          <dl className="detail-grid">
            <div>
              <dt>Support level</dt>
              <dd>{formatCurrency(overview.support_level)}</dd>
            </div>
            <div>
              <dt>Resistance level</dt>
              <dd>{formatCurrency(overview.resistance_level)}</dd>
            </div>
          </dl>
        </article>

        <article className="panel timeline-panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Scenario history</p>
              <h2>Recent directional states</h2>
            </div>
            <button
              type="button"
              className="ghost-button"
              onClick={() => setSelectedScenarioObservedAtMs(null)}
            >
              Follow latest
            </button>
          </div>

          <div className="timeline-list">
            {scenarios.map((scenario) => (
              <button
                key={scenario.observed_at_ms}
                type="button"
                className={`timeline-item timeline-button${projectedScenario.observed_at_ms === scenario.observed_at_ms ? ' timeline-item-active' : ''}`}
                onClick={() => setSelectedScenarioObservedAtMs(scenario.observed_at_ms)}
              >
                <div className="timeline-meta">
                  <span>{formatTimestamp(scenario.observed_at_ms)}</span>
                  <span className={`tag ${directionTone(scenario.expected_direction)}`}>
                    {scenario.expected_direction}
                  </span>
                </div>
                <p>{scenario.explanation}</p>
                <div className="timeline-probabilities">
                  <span>B {formatProbability(scenario.bull_probability)}</span>
                  <span>N {formatProbability(scenario.base_probability)}</span>
                  <span>R {formatProbability(scenario.bear_probability)}</span>
                </div>
              </button>
            ))}
          </div>
        </article>

        <article className="panel alerts-panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Alert tape</p>
              <h2>Latest state transitions</h2>
            </div>
            <span className="panel-header-meta-inline">Click an alert to project its scenario</span>
          </div>

          <div className="timeline-list">
            {alerts.length === 0 ? (
              <p className="empty-state">No alerts have been generated yet.</p>
            ) : (
              alerts.map((alert) => {
                const canProjectScenario = hasMatchingScenario(scenarios, alert.triggered_at_ms)

                return (
                <button
                  key={`${alert.alert_type}-${alert.triggered_at_ms}`}
                  type="button"
                  className={`timeline-item timeline-button${projectedScenario.observed_at_ms === alert.triggered_at_ms ? ' timeline-item-active' : ''}${!canProjectScenario ? ' timeline-item-muted' : ''}`}
                  onClick={() => {
                    if (canProjectScenario) {
                      setSelectedScenarioObservedAtMs(alert.triggered_at_ms)
                    }
                  }}
                  disabled={!canProjectScenario}
                >
                  <div className="timeline-meta">
                    <span>{formatTimestamp(alert.triggered_at_ms)}</span>
                    <span className={`tag ${severityTone(alert.severity)}`}>{alert.severity}</span>
                  </div>
                  <p>{alert.message}</p>
                  <div className="timeline-probabilities">
                    <span>{alert.alert_type.replace(/_/g, ' ')}</span>
                    <span>
                      {canProjectScenario
                        ? projectedScenario.observed_at_ms === alert.triggered_at_ms
                          ? 'projected'
                          : 'focus chart'
                        : alert.is_acknowledged
                          ? 'acknowledged'
                          : 'open'}
                    </span>
                  </div>
                </button>
              )})
            )}
          </div>
        </article>
      </section>

      {error ? <div className="error-banner">Latest refresh failed: {error}</div> : null}
    </main>
  )
}

export default App
