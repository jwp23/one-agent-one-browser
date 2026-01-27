.PHONY: test build

test:
	cargo test --locked --release

build:
	cargo build --release --locked --bin one-agent-one-browser

