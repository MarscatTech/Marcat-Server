BUILD_ENV := rust

.PHONY: build-wasm build-did

lint:
	@cargo fmt
	@cargo clippy --all-targets --all-features

fix:
	@cargo clippy --fix --workspace --tests --allow-dirty

# cargo install ic-wasm
build-wasm:
	@cargo build --release --target wasm32-unknown-unknown --package app-server

# cargo install candid-extractor
build-did:
	candid-extractor target/wasm32-unknown-unknown/release/app_server.wasm > src/app-server/app-server.did

set_info:
	dfx deploy app-server --network ic