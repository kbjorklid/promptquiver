check: fmt-check lint test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all --all-targets --all-features -- -D warnings

test:
    cargo test --all-features

build:
    cargo build --release

install-hooks:
    cp .githooks/pre-push .git/hooks/pre-push
    chmod +x .git/hooks/pre-push
    @echo "Hooks installed."
