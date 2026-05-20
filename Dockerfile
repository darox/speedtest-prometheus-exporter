# Multi-arch Dockerfile: arm64 + amd64
# Build: docker buildx build --platform linux/arm64,linux/amd64 -t speedtest-exporter .

# Stage 1: Build Rust binary (statically linked with musl)
# Pin manifest list: rust:1.95-slim
FROM docker.io/library/rust:1.95-slim@sha256:5021128d455987e7e7d6586bd7288fa876614821292614acbb761c21fc1ebb15 AS builder

ARG TARGETARCH
RUN rustup component add clippy rustfmt && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        rustup target add aarch64-unknown-linux-musl; \
    elif [ "$TARGETARCH" = "amd64" ]; then \
        rustup target add x86_64-unknown-linux-musl; \
    else \
        echo "Unsupported arch: $TARGETARCH" && exit 1; \
    fi

WORKDIR /app
COPY . .
RUN if [ "$TARGETARCH" = "arm64" ]; then \
        cargo build --release --target aarch64-unknown-linux-musl; \
    else \
        cargo build --release --target x86_64-unknown-linux-musl; \
    fi

# Stage 2: Test
FROM builder AS test
RUN cargo test

# Stage 2b: Audit tool (cached independently of source)
FROM docker.io/library/rust:1.95-slim@sha256:5021128d455987e7e7d6586bd7288fa876614821292614acbb761c21fc1ebb15 AS audit-tool
RUN cargo install cargo-audit

# Stage 2c: Audit (runs against mounted source)
FROM audit-tool AS audit

# Stage 3: Download Ookla CLI v1.2.0 (pinned, statically linked)
# Pin manifest list: debian:bookworm-slim
FROM docker.io/library/debian:bookworm-slim@sha256:67b30a61dc87758f0caf819646104f29ecbda97d920aaf5edc834128ac8493d3 AS ookla-downloader
ARG TARGETARCH
RUN apt-get update && apt-get install -y --no-install-recommends wget ca-certificates && \
    rm -rf /var/lib/apt/lists/*
RUN if [ "$TARGETARCH" = "arm64" ]; then \
        OOGLA_ARCH="aarch64"; \
        OOGLA_SHA="3953d231da3783e2bf8904b6dd72767c5c6e533e163d3742fd0437affa431bd3"; \
    elif [ "$TARGETARCH" = "amd64" ]; then \
        OOGLA_ARCH="x86_64"; \
        OOGLA_SHA="5690596c54ff9bed63fa3732f818a05dbc2db19ad36ed68f21ca5f64d5cfeeb7"; \
    else \
        echo "Unsupported arch: $TARGETARCH" && exit 1; \
    fi && \
    wget -qO/speedtest.tgz "https://install.speedtest.net/app/cli/ookla-speedtest-1.2.0-linux-${OOGLA_ARCH}.tgz" && \
    echo "${OOGLA_SHA}  /speedtest.tgz" | sha256sum -c - && \
    tar xzf /speedtest.tgz -C /usr/local/bin/ && \
    rm /speedtest.tgz

# Stage 4: Copy binary to fixed path (needed since distroless has no shell)
# Pin manifest list: debian:bookworm-slim
FROM docker.io/library/debian:bookworm-slim@sha256:67b30a61dc87758f0caf819646104f29ecbda97d920aaf5edc834128ac8493d3 AS assemble
ARG TARGETARCH
COPY --from=builder /app/target /app/target
RUN if [ "$TARGETARCH" = "arm64" ]; then \
        MUSL_DIR="aarch64-unknown-linux-musl"; \
    else \
        MUSL_DIR="x86_64-unknown-linux-musl"; \
    fi && \
    mkdir -p /out && \
    cp /app/target/${MUSL_DIR}/release/speedtest-exporter /out/speedtest-exporter
COPY --from=ookla-downloader /usr/local/bin/speedtest /out/speedtest
COPY --from=ookla-downloader /etc/ssl/certs /out/ssl/certs

# Stage 5: Minimal runtime (no shell, no glibc)
# Pin manifest list: gcr.io/distroless/static
FROM gcr.io/distroless/static@sha256:3592aa8171c77482f62bbc4164e6a2d141c6122554ace66e5cc910cadb961ff0

COPY --from=assemble /out/speedtest-exporter /speedtest-exporter
COPY --from=assemble /out/speedtest /usr/local/bin/speedtest
COPY --from=assemble /out/ssl/certs /etc/ssl/certs

ENTRYPOINT ["/speedtest-exporter"]
