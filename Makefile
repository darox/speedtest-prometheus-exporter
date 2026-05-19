.PHONY: lint fmt test docker docker-test helm-lint scan audit \
	release release-check release-build release-push release-update-chart release-publish-chart release-tag \
	kind-create kind-load kind-deploy kind-destroy kind-clean

VERSION ?=

lint:
	docker run --rm --entrypoint "" speedtest-exporter:builder cargo clippy -- -D warnings

fmt:
	docker run --rm --entrypoint "" speedtest-exporter:builder cargo fmt --check

test:
	docker build --target test -t speedtest-exporter:test .

docker:
	docker build -t speedtest-exporter .

docker-multiarch:
	docker buildx build --platform linux/amd64,linux/arm64 -t speedtest-exporter .

helm-lint:
	helm lint ./chart
	helm template ./chart > /dev/null

scan:
	docker run --rm ghcr.io/aquasecurity/trivy@sha256:be1190afcb28352bfddc4ddeb71470835d16462af68d310f9f4bca710961a41e image --severity CRITICAL,HIGH,MEDIUM speedtest-exporter

audit:
	docker run --rm -v $(shell pwd):/app -w /app \
		rust:1.95-slim \
		sh -c 'cargo install cargo-audit --locked 2>/dev/null && cargo audit'

# ── Release ──────────────────────────────────────────────
# Ordered pipeline: validate → build → update → publish → push → tag
# No irreversible action occurs before all validation passes.

release: release-check release-build release-update-chart release-publish-chart release-push release-tag

release-check:
	@if [ -z "$${VERSION}" ]; then echo "Usage: make release VERSION=v0.0.X"; exit 1; fi
	@if [ -n "$$(git status --short)" ]; then echo "Working tree dirty"; exit 1; fi
	@if [ "$$(git branch --show-current)" != "main" ]; then echo "Must be on main branch"; exit 1; fi
	docker build -t speedtest-exporter .
	$(MAKE) lint
	$(MAKE) fmt
	$(MAKE) test
	$(MAKE) scan
	$(MAKE) audit
	$(MAKE) helm-lint

release-build:
	@SHORT_SHA=$$(git rev-parse --short HEAD) && \
	echo "Building multi-arch image (sha-$${SHORT_SHA})" && \
	docker buildx build --platform linux/amd64,linux/arm64 -t speedtest-exporter:release .

release-update-chart:
	@CLEAN="$${VERSION#v}" && \
	sed -i '' "s/^version: .*/version: $${CLEAN}/" chart/Chart.yaml && \
	sed -i '' 's/^appVersion: .*/appVersion: "'$${CLEAN}'"/' chart/Chart.yaml && \
	sed -i '' "s|ghcr.io/darox/speedtest-exporter:[a-zA-Z0-9._-]*|ghcr.io/darox/speedtest-exporter:$${CLEAN}|g" README.md && \
	sed -i '' "s|ghcr.io/darox/speedtest-exporter:[a-zA-Z0-9._-]*|ghcr.io/darox/speedtest-exporter:$${CLEAN}|g" chart/README.md && \
	grep -q "version: $${CLEAN}" chart/Chart.yaml || { echo "Chart.yaml version mismatch"; exit 1; } && \
	grep -q 'appVersion: "$${CLEAN}"' chart/Chart.yaml || { echo "Chart.yaml appVersion mismatch"; exit 1; } && \
	grep -q "speedtest-exporter:$${CLEAN}" README.md || { echo "README.md image tag mismatch"; exit 1; } && \
	grep -q "speedtest-exporter:$${CLEAN}" chart/README.md || { echo "chart/README.md image tag mismatch"; exit 1; }

release-publish-chart:
	@CLEAN="$${VERSION#v}" && \
	WORKTREE="$$(mktemp -d)" && \
	git worktree add "$${WORKTREE}" gh-pages && \
	helm package chart && \
	mv speedtest-exporter-*.tgz "$${WORKTREE}/" && \
	(cd "$${WORKTREE}" && \
		helm repo index . --url https://darox.github.io/speedtest-prometheus-exporter --merge index.yaml && \
		git add speedtest-exporter-$${CLEAN}.tgz index.yaml && \
		git commit -m "Publish Helm chart v$${CLEAN}") && \
	(cd "$${WORKTREE}" && git push origin gh-pages) && \
	git worktree remove "$${WORKTREE}" && \
	rm -rf "$${WORKTREE}"

release-push:
	@IMAGE="ghcr.io/darox/speedtest-exporter" && \
	CLEAN="$${VERSION#v}" && \
	MINOR="$${CLEAN%.*}" && \
	MAJOR="$${CLEAN%%.*}" && \
	SHORT_SHA=$$(git rev-parse --short HEAD) && \
	echo "Pushing image tags: $${VERSION} $${CLEAN} $${MINOR} $${MAJOR} sha-$${SHORT_SHA}" && \
	docker tag speedtest-exporter:release "$${IMAGE}:$${VERSION}" && \
	docker tag speedtest-exporter:release "$${IMAGE}:$${CLEAN}" && \
	docker tag speedtest-exporter:release "$${IMAGE}:$${MINOR}" && \
	docker tag speedtest-exporter:release "$${IMAGE}:$${MAJOR}" && \
	docker tag speedtest-exporter:release "$${IMAGE}:sha-$${SHORT_SHA}" && \
	docker login ghcr.io -u darox -p "$$(gh auth token)" && \
	docker push "$${IMAGE}:$${VERSION}" && \
	docker push "$${IMAGE}:$${CLEAN}" && \
	docker push "$${IMAGE}:$${MINOR}" && \
	docker push "$${IMAGE}:$${MAJOR}" && \
	docker push "$${IMAGE}:sha-$${SHORT_SHA}"

release-tag:
	@git add chart/Chart.yaml README.md chart/README.md && \
	git commit -m "Release $${VERSION}" && \
	git tag "$${VERSION}" && \
	git push origin main "$${VERSION}"

# ── Local Kind Cluster ───────────────────────────────────
KIND_CLUSTER ?= speedtest-exporter

kind-create:
	@kind get clusters | grep -q '^$(KIND_CLUSTER)$$' || \
		kind create cluster --name $(KIND_CLUSTER) --wait 30s

kind-load:
	docker build -t speedtest-exporter:local . && \
		kind load docker-image speedtest-exporter:local --name $(KIND_CLUSTER)

kind-deploy:
	helm upgrade --install speedtest-exporter ./chart \
		-n speedtest-exporter \
		--create-namespace \
		--set image.repository=speedtest-exporter \
		--set image.tag=local \
		--set image.pullPolicy=IfNotPresent

kind-destroy:
	helm uninstall speedtest-exporter -n speedtest-exporter 2>/dev/null || true
	kubectl delete ns speedtest-exporter 2>/dev/null || true

kind-clean: kind-destroy
	kind delete cluster --name $(KIND_CLUSTER)
