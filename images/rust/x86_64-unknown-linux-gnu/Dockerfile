FROM rust:1.98.0@sha256:271849e998ffce5776454bbf98c5dc21baafc854ff8e566197908d3aca9a81e8

# renovate: datasource=github-releases depName=mozilla/sccache
ARG SCCACHE_VERSION=0.13.0

# Install sccache from pre-built binary
RUN wget -qO- "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar zxf - -C /tmp \
    && mv /tmp/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/cargo/bin/ \
    && rm -rf /tmp/sccache-*

# Verify installation
RUN sccache --version
