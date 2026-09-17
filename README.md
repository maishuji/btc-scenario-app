# AI Trader Workspace

## Local Development

The repository now supports a single root development entrypoint.

### First-time setup

```bash
npm install
cd frontend && npm install
```

### Start the API and frontend together

```bash
npm run dev
```

This starts:

- the Rust API with `cargo run --manifest-path rust/Cargo.toml -p market-intelligence-app -- serve-api`
- the Vite frontend on `http://127.0.0.1:5173`

The frontend proxies `/api` requests to the Rust API on `http://127.0.0.1:3000`.
The API now performs an initial Binance sync at startup and continues refreshing the local SQLite store in the background while it serves requests.

### Useful root commands

```bash
npm run build:frontend
npm run test:rust
```