.PHONY: lint fmt test docker docker-test helm-lint scan

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
	trivy image --severity CRITICAL,HIGH,MEDIUM speedtest-exporter
