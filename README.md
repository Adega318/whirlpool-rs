# Whirlpool

**A real-time, multi-exchange cryptocurrency market data aggregator written in Rust.**

Whirlpool connects to cryptocurrency exchanges, normalizes live market data, and aggregates order books into a unified market view — all from an interactive terminal UI powered by [Ratatui](https://ratatui.rs/).

## Roadmap

- [x] Establish the Ratatui application and terminal event loop.
- [x] Exchange implementations.
  - [x] Kraken.
  - [x] Binance
  - [ ] Coinbase.
  - [ ] Custom exchanges.
- [x] Normalize incoming market data.
- [x] Market aggregation engine.
- [x] Display best bid/ask and cross-exchange spreads.
- [ ] Robust reconnection and order book recovery.
- [ ] Historical event replay and performance benchmarks.

## Disclaimer

Whirlpool is a market data and software engineering project. Displayed spreads do not guarantee executable arbitrage opportunities or profitable trades. Exchange fees, liquidity, latency, and execution constraints must be considered.

## License

To be determined.
