.PHONY: all build binary dev test dist

all: build

build: dist

binary:
	cargo build --release --locked

dev:
	cargo run --locked

test:
	cargo test --locked

dist:
	./scripts/build-package.sh all
