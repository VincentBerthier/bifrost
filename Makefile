BINARY=bifrost
SOURCES=$(shell fd -e rs)
PGO_PATH=target/x86_64-unknown-linux-gnu/release

default: run_debug

all: checks tests pgo

./target/debug/$(BINARY): $(SOURCES) Cargo.toml
	cargo build

./target/release/$(BINARY): $(SOURCES) Cargo.toml
	cargo build --release

./target/profiling/$(BINARY): $(SOURCES) Cargo.toml
	cargo build --profile profiling

./$(PGO_PATH)/$(BINARY)-bolt-optimized: $(SOURCES) Cargo.toml
	echo '1' | doas tee /proc/sys/kernel/perf_event_paranoid
	cargo pgo build
	./$(PGO_PATH)/$(BINARY)
	# cargo pgo optimize
	cargo pgo bolt build --with-pgo
	./$(PGO_PATH)/$(BINARY)-bolt-instrumented
	cargo pgo bolt optimize --with-pgo
	cp ./$(PGO_PATH)/$(BINARY)-bolt-optimized ./target/orpailleur-pgo-bolt-optimized

bench: $(SOURCES) Cargo.toml
	cargo criterion

.PHONY = run
run: release
	./target/release/$(BINARY)

.PHONY = run_debug
run_debug: debug
	RUST_LOG=info,bifrost=debug ./target/debug/$(BINARY)

.PHONY = run_pgo
run_pgo: pgo
	./$(PGO_PATH)/$(BINARY)-bolt-optimized

.PHONY = time
time: release pgo
	perf stat -ddd -o target/perf.log -r 10 -B ./target/release/$(BINARY)
	perf stat -ddd -o target/perf.log --append -r 10 -B ./$(PGO_PATH)/$(BINARY)-bolt-optimized


.PHONY = release
release: ./target/release/$(BINARY)

.PHONY = debug
debug: ./target/debug/$(BINARY)

.PHONY = profiling
profiling: ./target/profiling/$(BINARY)

.PHONY = pgo
pgo: ./$(PGO_PATH)/$(BINARY)-bolt-optimized

.PHONY = tests
tests: $(SOURCES)
	cargo nextest run --all-features --all-targets
	cargo test --doc --all-features

.PHONY = mutants
mutants: $(SOURCES)
	cargo mutants --test-tool=nextest -e main.rs

.PHONY = coverage
coverage: $(SOURCES) Cargo.toml
	cargo llvm-cov clean --profraw-only
	cargo llvm-cov --no-report --locked --all-features nextest
	cargo llvm-cov --all-features --doc --no-report
	cargo llvm-cov report --doctests --ignore-filename-regex='(main.rs$$)' --json --output-path ./target/coverage.json
	llvm-cov-pretty --theme dracula --coverage-style gutter --skip-function-coverage ./target/coverage.json

.PHONY = doc
doc: $(SOURCES)
	cargo doc --no-deps --all-features

.PHONY = checks
checks:
	cargo deny check
	cargo audit
	cargo spellcheck --code 1 check
	cargo clippy --all-features --workspace --all-targets

.PHONY = flame
flame:
	cargo flamegraph --profile profiling --palette rust

.PHONY = samply
samply: profiling
	echo '1' | doas tee /proc/sys/kernel/perf_event_paranoid
	samply record ./target/profiling/$(BINARY)

.PHONY = samply-pgo
samply-pgo: pgo
	samply record ./target/x86_64-unknown-linux-gnu/release/$(BINARY)-bolt-optimized

.PHONY = clean
clean:
	cargo clean
	rm -r perf.data*
	rm -r flamegraph.svg
	rm -r profile.json
	rm -rf perf.*
