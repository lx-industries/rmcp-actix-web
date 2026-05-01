FROM rust:1.95.0@sha256:a9cfb755b33f5bb872610cbdb25da61f527416b28fc9c052bbce4bef93e7799a

# renovate: datasource=github-releases depName=mozilla/sccache
ARG SCCACHE_VERSION=0.13.0

# Install sccache from pre-built binary
RUN wget -qO- "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar zxf - -C /tmp \
    && mv /tmp/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/cargo/bin/ \
    && rm -rf /tmp/sccache-*

# Verify installation
RUN sccache --version
