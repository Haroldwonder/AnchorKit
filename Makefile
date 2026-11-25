.PHONY: build test testnet-integration lint fmt clean deploy-testnet help build-wasm storybook validate coverage bench bindings bindings-check

## build: Build the contract in release mode
build:
	cargo build --release

## test: Run all Rust tests
test:
	cargo test

## testnet-integration: Run SEP-6 testnet integration tests
testnet-integration:
	cargo test --features testnet-integration

## bench: Run criterion benchmarks for contract hot paths
bench:
	cargo bench

## lint: Run clippy with warnings as errors
lint:
	cargo clippy -- -D warnings

## build-wasm: Build the contract WASM artifact used for deployment
build-wasm:
	cargo build --release --target wasm32-unknown-unknown

## bindings: Regenerate the typed TS contract client from the built WASM
bindings: build-wasm
	stellar contract bindings typescript \
		--wasm target/wasm32-unknown-unknown/release/anchorkit.wasm \
		--output-dir ui/src/contracts/anchorkit \
		--overwrite

## bindings-check: Fail if committed bindings drift from the contract spec
bindings-check: bindings
	@git diff --exit-code -- ui/src/contracts/anchorkit \
		|| (echo "Committed contract bindings are out of date. Run 'make bindings' and commit the result." && exit 1)

## storybook: Run Storybook (UI components)
storybook:
	cd ui && npm run storybook

## validate: Run repo validation (configs + Rust validation)
validate:
	./validate_all.sh

## coverage: Run Rust + UI coverage (best-effort)
coverage:
	cargo tarpaulin --version >/dev/null 2>&1 || true
	@echo "(1/2) Rust coverage"
	@cargo tarpaulin --ignore-tests 2>/dev/null || cargo test
	@echo "(2/2) UI coverage"
	cd ui && npm run test:coverage

## fmt: Format code with rustfmt
fmt:
	cargo fmt

## clean: Remove build artifacts
clean:
	cargo clean

## deploy-testnet: Deploy contract to Stellar testnet
deploy-testnet: build-wasm
	soroban contract deploy \
		--wasm target/wasm32-unknown-unknown/release/anchorkit.wasm \
		--network testnet

## help: Show this help message
help:
	@grep -E '^## ' Makefile | sed 's/## //'

