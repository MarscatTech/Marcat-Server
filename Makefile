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
	@cargo build --release --target wasm32-unknown-unknown --package sticker-server
	@cargo build --release --target wasm32-unknown-unknown --package feed-server

# cargo install candid-extractor
build-did:
	candid-extractor target/wasm32-unknown-unknown/release/app_server.wasm > src/app-server/app-server.did
	candid-extractor target/wasm32-unknown-unknown/release/sticker_server.wasm > src/sticker-server/sticker-server.did
	candid-extractor target/wasm32-unknown-unknown/release/feed_server.wasm > src/feed-server/feed-server.did

set_info:
	dfx deploy app-server --network ic