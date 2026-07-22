# Trading Systems & Quantitative Finance

## CRITICAL: Financial Code Precision Rules

```
1. NEVER use float/double for money — use Decimal (Python), BigDecimal (Java), decimal.js (JS)
2. ALL monetary arithmetic: multiply first, divide last, round at the END
3. Rounding: ROUND_HALF_EVEN (banker's rounding) unless regulation specifies otherwise
4. Timestamps: ALWAYS UTC internally, timezone-convert only for display
5. Price precision: stock = 2 decimals, forex = 4-5 pips, crypto = 8 decimals
6. Every calculation MUST be reproducible (seed random, pin library versions)
```

## Backtesting Framework

### Architecture
```python
class Backtest:
    def __init__(self, strategy, data, initial_capital, commission=0.001):
        self.strategy = strategy
        self.data = data
        self.capital = Decimal(str(initial_capital))
        self.commission = Decimal(str(commission))
        self.positions = {}
        self.trades = []
        self.equity_curve = []

    def run(self):
        for i, bar in enumerate(self.data):
            # Strategy sees ONLY past data (no look-ahead)
            visible = self.data[:i+1]
            signals = self.strategy.generate_signals(visible)
            self._execute_signals(signals, bar)
            self.equity_curve.append(self._portfolio_value(bar))
```

### Bias Prevention Checklist
| Bias | Prevention |
|------|-----------|
| Look-ahead | Strategy receives only data[:current_index], NEVER future data |
| Survivorship | Include delisted/bankrupt stocks in historical data |
| Overfitting | Out-of-sample test (60/20/20 train/val/test), walk-forward |
| Transaction costs | Always deduct commission + slippage (0.05-0.1% minimum) |
| Fill assumption | Use OHLC: buy at next-bar open, not current close |

### Risk Management (MUST implement)
```python
# Position sizing — never risk > 1-2% of capital per trade
def position_size(capital, entry, stop_loss, risk_pct=0.01):
    risk_per_share = abs(entry - stop_loss)
    if risk_per_share == 0:
        return 0
    max_risk = capital * Decimal(str(risk_pct))
    return int(max_risk / risk_per_share)

# Portfolio-level limits
MAX_POSITIONS = 20
MAX_SECTOR_EXPOSURE = Decimal('0.25')  # 25% max in one sector
MAX_DRAWDOWN_HALT = Decimal('0.15')    # stop trading at 15% drawdown
```

### Performance Metrics (Always Report)
```python
def calc_metrics(equity_curve, risk_free_rate=0.02):
    returns = pd.Series(equity_curve).pct_change().dropna()
    return {
        'total_return': (equity_curve[-1] / equity_curve[0]) - 1,
        'cagr': ((equity_curve[-1] / equity_curve[0]) ** (252/len(returns))) - 1,
        'sharpe': (returns.mean() - risk_free_rate/252) / returns.std() * np.sqrt(252),
        'sortino': (returns.mean() - risk_free_rate/252) / returns[returns<0].std() * np.sqrt(252),
        'max_drawdown': ((equity_curve / np.maximum.accumulate(equity_curve)) - 1).min(),
        'win_rate': (returns > 0).mean(),
        'profit_factor': returns[returns>0].sum() / abs(returns[returns<0].sum()),
        'calmar': None,  # CAGR / max_drawdown
    }
```

## Time-Series Forecasting

### Data Preparation
```python
# 1. Check stationarity (MUST be stationary for most models)
from statsmodels.tsa.stattools import adfuller
result = adfuller(series)
if result[1] > 0.05:  # p-value > 5% → non-stationary
    series = series.diff().dropna()  # difference until stationary

# 2. Feature engineering for financial time series
features = pd.DataFrame({
    'returns': prices.pct_change(),
    'log_returns': np.log(prices / prices.shift(1)),
    'volatility_20d': prices.pct_change().rolling(20).std(),
    'sma_50': prices.rolling(50).mean(),
    'sma_200': prices.rolling(200).mean(),
    'rsi_14': calc_rsi(prices, 14),
    'macd': calc_macd(prices),
    'volume_sma': volume.rolling(20).mean(),
    'bb_upper': sma_20 + 2 * std_20,
    'bb_lower': sma_20 - 2 * std_20,
})

# 3. Train/test split — NEVER shuffle time series
train = data[:'2023-06-30']
val = data['2023-07-01':'2023-12-31']
test = data['2024-01-01':]
```

### Model Selection
| Data Size | Model | When |
|-----------|-------|------|
| < 500 points | ARIMA/GARCH | Simple trend + volatility |
| 500-5000 | XGBoost + engineered features | Tabular with indicators |
| > 5000 | LSTM / Transformer | Complex patterns, multivariate |

## Portfolio Optimization

### Modern Portfolio Theory
```python
from scipy.optimize import minimize

def optimize_portfolio(returns, risk_free=0.02):
    n = len(returns.columns)

    def neg_sharpe(weights):
        port_ret = np.dot(weights, returns.mean()) * 252
        port_vol = np.sqrt(np.dot(weights.T, np.dot(returns.cov() * 252, weights)))
        return -(port_ret - risk_free) / port_vol

    constraints = [
        {'type': 'eq', 'fun': lambda w: np.sum(w) - 1},  # weights sum to 1
    ]
    bounds = [(0, 0.25)] * n  # max 25% per asset

    result = minimize(neg_sharpe, [1/n]*n, bounds=bounds, constraints=constraints)
    return result.x
```

### Risk Parity
```python
def risk_parity(cov_matrix):
    n = cov_matrix.shape[0]

    def risk_budget_error(weights):
        port_vol = np.sqrt(weights @ cov_matrix @ weights)
        marginal_contrib = cov_matrix @ weights
        risk_contrib = weights * marginal_contrib / port_vol
        target = port_vol / n  # equal risk contribution
        return np.sum((risk_contrib - target) ** 2)

    result = minimize(risk_budget_error, [1/n]*n,
                     bounds=[(0.01, 1)]*n,
                     constraints={'type': 'eq', 'fun': lambda w: sum(w)-1})
    return result.x
```

## Order Book & Market Microstructure

### Order Book Data Structure
```python
from sortedcontainers import SortedDict

class OrderBook:
    def __init__(self):
        self.bids = SortedDict()  # price → [(qty, order_id, ts), ...]
        self.asks = SortedDict()  # price → [(qty, order_id, ts), ...]

    def best_bid(self):
        return self.bids.peekitem(-1)[0] if self.bids else None

    def best_ask(self):
        return self.asks.peekitem(0)[0] if self.asks else None

    def spread(self):
        bb, ba = self.best_bid(), self.best_ask()
        return (ba - bb) if bb and ba else None

    def mid_price(self):
        bb, ba = self.best_bid(), self.best_ask()
        return (bb + ba) / 2 if bb and ba else None

    def vwap(self, side, depth):
        """Volume-weighted average price for top `depth` levels"""
        book = self.asks if side == 'buy' else self.bids
        total_qty, total_value = Decimal(0), Decimal(0)
        for price, orders in (book.items() if side == 'buy' else reversed(book.items())):
            for qty, _, _ in orders:
                total_qty += qty
                total_value += price * qty
                if total_qty >= depth:
                    return total_value / total_qty
        return total_value / total_qty if total_qty > 0 else None
```

## Financial Calculations (Reference)

```python
from decimal import Decimal, ROUND_HALF_EVEN

def compound_interest(principal, rate, periods, compounds_per_period=1):
    """A = P(1 + r/n)^(nt)"""
    r = Decimal(str(rate))
    n = Decimal(str(compounds_per_period))
    t = Decimal(str(periods))
    return Decimal(str(principal)) * (1 + r/n) ** (n * t)

def npv(rate, cashflows):
    """Net Present Value"""
    r = Decimal(str(rate))
    return sum(Decimal(str(cf)) / (1 + r) ** i for i, cf in enumerate(cashflows))

def irr(cashflows, guess=0.1, tol=1e-6, max_iter=100):
    """Internal Rate of Return (Newton's method)"""
    rate = guess
    for _ in range(max_iter):
        npv_val = sum(cf / (1 + rate) ** i for i, cf in enumerate(cashflows))
        dnpv = sum(-i * cf / (1 + rate) ** (i+1) for i, cf in enumerate(cashflows))
        if abs(dnpv) < 1e-12:
            break
        rate -= npv_val / dnpv
        if abs(npv_val) < tol:
            break
    return rate

def amortization_schedule(principal, annual_rate, months):
    """Fixed-rate loan amortization"""
    r = Decimal(str(annual_rate)) / 12
    p = Decimal(str(principal))
    payment = p * (r * (1+r)**months) / ((1+r)**months - 1)
    schedule = []
    balance = p
    for month in range(1, months + 1):
        interest = (balance * r).quantize(Decimal('0.01'), ROUND_HALF_EVEN)
        principal_paid = payment - interest
        balance -= principal_paid
        schedule.append({'month': month, 'payment': payment, 'interest': interest,
                        'principal': principal_paid, 'balance': max(balance, Decimal(0))})
    return schedule
```
