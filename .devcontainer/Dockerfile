# syntax=docker/dockerfile:1
FROM rust:1.94.1@sha256:a449cfcee87dad556a8fad7e3f7922d6ef77dfb4ced4224b7cc1413f6f145bd2

# -- Environment (set early so tool installers use the right paths) -----------
ENV HOME=/tmp/home
ENV PATH="${HOME}/.local/bin:${PATH}"

# -- NPM supply-chain hardening ------------------------------------------------
# Block postinstall scripts and require 24h package age for supply-chain safety.
ENV NPM_CONFIG_IGNORE_SCRIPTS=true
ENV NPM_CONFIG_MINIMUM_RELEASE_AGE=1440

# -- Forge CLI config directory ------------------------------------------------
ENV GLAB_CONFIG_DIR=/tmp/glab-config

# -- Rust components (clippy, rustfmt) -----------------------------------------
RUN rustup component add clippy rustfmt

# -- Node.js (needed for JS interop tests) ------------------------------------
# renovate: datasource=node-version depName=node
ARG NODE_VERSION="22.14.0"
RUN arch="$(dpkg --print-architecture)" \
    && case "$arch" in amd64) node_arch=x64;; arm64) node_arch=arm64;; *) echo "Unsupported arch: $arch" >&2; exit 1;; esac \
    && curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${node_arch}.tar.xz" \
       | tar -xJ --strip-components=1 -C /usr/local \
    && node --version && npm --version

# -- System packages -----------------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
        openssh-client \
        iptables ipset iproute2 dnsutils \
    && rm -rf /var/lib/apt/lists/*

# -- gosu (privilege drop for firewalled mode) ---------------------------------
# renovate: datasource=github-releases depName=tianon/gosu
ARG GOSU_VERSION="1.19"
RUN arch="$(dpkg --print-architecture)" \
    && curl -fsSL "https://github.com/tianon/gosu/releases/download/${GOSU_VERSION}/gosu-${arch}" -o /usr/local/bin/gosu \
    && chmod +x /usr/local/bin/gosu \
    && gosu --version

# -- glab CLI ------------------------------------------------------------------
# renovate: datasource=gitlab-releases depName=gitlab-org/cli registryUrl=https://gitlab.com
ARG GLAB_VERSION="1.87.0"
RUN curl -fsSL "https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/glab_${GLAB_VERSION}_linux_amd64.deb" -o /tmp/glab.deb \
    && dpkg -i /tmp/glab.deb \
    && rm /tmp/glab.deb \
    && glab --version \
    && rm -rf /tmp/glab-config \
    && mkdir -m 1777 /tmp/glab-config

# -- Claude Code (native binary, auto-updates) --------------------------------
RUN curl -fsSL https://claude.ai/install.sh | bash \
    && claude --version

# -- Git and SSH configuration -------------------------------------------------
RUN git config --system safe.directory '*'

RUN mkdir -p /etc/ssh \
    && ssh-keyscan -t ecdsa,rsa,ed25519 gitlab.com github.com >> /etc/ssh/ssh_known_hosts 2>/dev/null

# -- Non-root user (for remoteUser + updateRemoteUserUID) --------------------
RUN groupadd --gid 1000 dev \
 && useradd --uid 1000 --gid 1000 --no-create-home --home-dir /tmp/home --shell /bin/bash dev

# -- Firewall files ------------------------------------------------------------
COPY firewall.sh /usr/local/bin/firewall.sh
RUN chmod +x /usr/local/bin/firewall.sh

COPY firewall-allowlist.txt /usr/local/share/firewall-allowlist.txt

RUN printf '#!/bin/bash\ncat /tmp/firewall-allowed-ips.txt 2>/dev/null || echo "Firewall not active"\n' \
        > /usr/local/bin/firewall-list \
    && chmod +x /usr/local/bin/firewall-list

# -- Permissions ---------------------------------------------------------------
RUN chmod -R 1777 /tmp/home

RUN chmod 0666 /etc/passwd

# -- Entrypoint ----------------------------------------------------------------
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["bash"]
