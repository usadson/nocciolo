.PHONY: build

build:
	cargo bootimage

run: build
	cargo run

setup-tools:
	rustup target add x86_64-unknown-none
	cargo install bootimage
