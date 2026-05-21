.PHONY: build test bench lint format docker-build k8s-deploy load-test

build:
	cargo build --release

test:
	cargo test --workspace

bench:
	cargo bench --workspace

lint:
	cargo clippy --workspace --all-targets -- -D warnings

format:
	cargo +nightly fmt --all

docker-build:
	docker build -t vortex-proxy:latest .

k8s-deploy:
	kubectl apply -f k8s/

load-test:
	./scripts/perf-test.sh 30s 10000 http://127.0.0.1:8443
