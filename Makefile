.PHONY: lint fmt test docker docker-test helm-lint scan audit \
	release release-check release-build release-scan release-update-chart release-publish-chart release-tag release-push \
	builder-setup \
	kind-create kind-load kind-deploy kind-destroy kind-clean

VERSION ?=

builder:
	docker build --target builder -t speedtest-exporter:builder .

lint: builder
	docker run --rm --entrypoint "" speedtest-exporter:builder cargo clippy -- -D warnings

fmt: builder
	docker run --rm --entrypoint "" speedtest-exporter:builder cargo fmt --check

test:
	docker build --target test -t speedtest-exporter:test .

docker:
	docker build -t speedtest-exporter .

docker-multiarch:
	docker buildx build --builder multiarch --platform linux/amd64,linux/arm64 --load -t speedtest-exporter .

helm-lint:
	helm lint ./chart
	helm template ./chart > /dev/null

scan:
	docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
		ghcr.io/aquasecurity/trivy@sha256:be1190afcb28352bfddc4ddeb71470835d16462af68d310f9f4bca710961a41e \
		image --severity CRITICAL,HIGH,MEDIUM speedtest-exporter

audit:
	docker build --target audit -t speedtest-exporter:audit . && \
	docker run --rm -v $(shell pwd):/app -w /app speedtest-exporter:audit cargo audit

builder-setup:
	@if ! docker buildx ls | grep -q 'multiarch'; then \
		docker buildx create --name multiarch --driver docker-container --use; \
	fi
	docker buildx inspect --bootstrap multiarch >/dev/null 2>&1

# ── Release ──────────────────────────────────────────────
# Ordered pipeline: validate → build → scan → update → tag → publish → push
# All local operations complete before any external push.
# Tag variables computed once by the orchestrator, exported to sub-targets.

release:
	@if [ -z "$${VERSION}" ]; then echo "Usage: make release VERSION=v0.0.X"; exit 1; fi
	@CLEAN="$${VERSION#v}" && \
	MINOR="$${CLEAN%.*}" && \
	MAJOR="$${CLEAN%%.*}" && \
	SHORT_SHA=$$(git rev-parse --short HEAD) && \
	export RELEASE_CLEAN="$${CLEAN}" RELEASE_MINOR="$${MINOR}" RELEASE_MAJOR="$${MAJOR}" RELEASE_SHA="$${SHORT_SHA}" && \
	$(MAKE) builder-setup && \
	$(MAKE) release-check && \
	$(MAKE) release-build && \
	$(MAKE) release-scan && \
	$(MAKE) release-update-chart && \
	$(MAKE) release-tag && \
	$(MAKE) release-publish-chart && \
	$(MAKE) release-push

release-check:
	@if [ -z "$${VERSION}" ]; then echo "Usage: make release VERSION=v0.0.X"; exit 1; fi
	@if [ -n "$$(git status --short)" ]; then echo "Working tree dirty"; exit 1; fi
	@if [ "$$(git branch --show-current)" != "main" ]; then echo "Must be on main branch"; exit 1; fi
	$(MAKE) lint
	$(MAKE) fmt
	$(MAKE) test
	$(MAKE) audit
	$(MAKE) helm-lint

release-build:
	@SHA="$${RELEASE_SHA:-$$(git rev-parse --short HEAD)}" && \
	echo "Building local image for scan (sha-$${SHA})" && \
	docker build -t speedtest-exporter:release .

release-scan:
	docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
		ghcr.io/aquasecurity/trivy@sha256:be1190afcb28352bfddc4ddeb71470835d16462af68d310f9f4bca710961a41e \
		image --severity CRITICAL,HIGH,MEDIUM speedtest-exporter:release

release-update-chart:
	@CLEAN="$${RELEASE_CLEAN:-$${VERSION#v}}" && \
	sed -i '' "s/^version: .*/version: $${CLEAN}/" chart/Chart.yaml && \
	sed -i '' 's/^appVersion: .*/appVersion: "'$${CLEAN}'"/' chart/Chart.yaml && \
	sed -i '' "s|ghcr.io/darox/speedtest-prometheus-exporter:[a-zA-Z0-9._-]*|ghcr.io/darox/speedtest-prometheus-exporter:$${CLEAN}|g" README.md && \
	grep -q "version: $${CLEAN}" chart/Chart.yaml || { echo "Chart.yaml version mismatch"; exit 1; } && \
	grep -q "appVersion: \"$${CLEAN}\"" chart/Chart.yaml || { echo "Chart.yaml appVersion mismatch"; exit 1; } && \
	grep -q "speedtest-prometheus-exporter:$${CLEAN}" README.md || { echo "README.md image tag mismatch"; exit 1; }

release-tag:
	@CLEAN="$${RELEASE_CLEAN:-$${VERSION#v}}" && \
	if git tag -l "$${VERSION}" | grep -q "$${VERSION}"; then echo "Tag $${VERSION} already exists"; exit 1; fi && \
	git add chart/Chart.yaml README.md && \
	git diff --cached --stat && \
	echo "Committing and tagging $${VERSION} (local only)" && \
	git commit -m "Release $${VERSION}" && \
	git tag "$${VERSION}"

release-publish-chart:
	@CLEAN="$${RELEASE_CLEAN:-$${VERSION#v}}" && \
	WORKTREE="$$(mktemp -d)" && \
	git worktree add "$${WORKTREE}" gh-pages && \
	trap 'echo "Cleaning up worktree..."; git worktree remove -f "$${WORKTREE}" 2>/dev/null; rm -rf "$${WORKTREE}"' EXIT && \
	helm package chart && \
	mv speedtest-exporter-*.tgz "$${WORKTREE}/" && \
	(cd "$${WORKTREE}" && \
		helm repo index . --url https://darox.github.io/speedtest-prometheus-exporter --merge index.yaml && \
		git add speedtest-exporter-$${CLEAN}.tgz index.yaml && \
		git commit -m "Publish Helm chart v$${CLEAN}") && \
	(cd "$${WORKTREE}" && git push origin gh-pages) && \
	git worktree remove "$${WORKTREE}" && \
	rm -rf "$${WORKTREE}" && \
	trap - EXIT

release-push:
	@IMAGE="ghcr.io/darox/speedtest-prometheus-exporter" && \
	CLEAN="$${RELEASE_CLEAN:-$${VERSION#v}}" && \
	MINOR="$${RELEASE_MINOR:-$${CLEAN%.*}}" && \
	MAJOR="$${RELEASE_MAJOR:-$${CLEAN%%.*}}" && \
	SHA="$${RELEASE_SHA:-$$(git rev-parse --short HEAD)}" && \
	echo "Pushing image tags: $${VERSION} $${CLEAN} $${MINOR} $${MAJOR} sha-$${SHA}" && \
	docker login ghcr.io -u darox -p "$$(gh auth token)" && \
	docker buildx build --builder multiarch --platform linux/amd64,linux/arm64 --push \
		-t "$${IMAGE}:$${VERSION}" \
		-t "$${IMAGE}:$${CLEAN}" \
		-t "$${IMAGE}:$${MINOR}" \
		-t "$${IMAGE}:$${MAJOR}" \
		-t "$${IMAGE}:sha-$${SHA}" . && \
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
