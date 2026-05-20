lint:
    cargo fmt --manifest-path stl-base/Cargo.toml --check
    cargo clippy --manifest-path stl-base/Cargo.toml --all-targets -- -D warnings

_check-nextest:
    @if ! cargo nextest --version >/dev/null 2>&1; then \
      echo "error: cargo-nextest is not installed"; \
      echo "install it with: cargo install cargo-nextest --locked"; \
      exit 1; \
    fi

test: _check-nextest
    cargo nextest run --manifest-path stl-base/Cargo.toml --no-fail-fast

test-unit: _check-nextest
    cargo nextest run --manifest-path stl-base/Cargo.toml --lib --no-fail-fast

test-language: _check-nextest
    cargo nextest run --manifest-path stl-base/Cargo.toml --test language --no-fail-fast

repl:
    cargo run --manifest-path stl-base/Cargo.toml
