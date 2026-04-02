# MarsCat-APP-Server

Backend canister for the MarsCat application ecosystem.

## Prerequisites

- [DFX SDK](https://internetcomputer.org/docs/current/developer-docs/setup/install) >= 0.30.1
- [Rust](https://www.rust-lang.org/tools/install) with `wasm32-unknown-unknown` target

```bash
rustup target add wasm32-unknown-unknown
cargo install ic-wasm
cargo install candid-extractor
```

## Build

```bash
# Format & lint
make lint

# Apply clippy fixes
make fix

# Build WASM canister
make build-wasm

# Generate Candid interface from WASM
make build-did
```

## Deploy

### Local

```bash
dfx start --background
dfx deploy app-server
```

### Mainnet

```bash
dfx deploy app-server --network ic
```

## License

MIT
