DOCKER ?= docker

.PHONY: infra infra-down api worker seed web mobile test test-api test-web test-mobile lint

infra:            ## start postgres, redis, minio
	$(DOCKER) compose up -d postgres redis minio

infra-down:
	$(DOCKER) compose down

api:              ## run the API on :8080
	cd api && cargo run -p reelearn-api

worker:           ## run the worker + scheduler
	cd api && cargo run -p reelearn-worker

seed:             ## load the demo tenant (Al Wadi Retail)
	cd api && cargo run -p reelearn-api -- seed

web:              ## run the console on :3000
	cd web && pnpm dev

mobile:           ## run the app on the connected device/simulator
	cd mobile && flutter run

test: test-api test-web test-mobile

test-api:
	cd api && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace

test-web:
	cd web && pnpm lint && pnpm typecheck && pnpm test && pnpm e2e

test-mobile:
	cd mobile && dart format --set-exit-if-changed lib test && flutter analyze && flutter test

lint:
	npx --yes @redocly/cli@latest lint openapi/openapi.yaml
