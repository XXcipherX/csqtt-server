# syntax=docker/dockerfile:1
# SPDX-FileCopyrightText: 2026 amurcanov
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

ARG RUST_VERSION=1.97.1

FROM rust:${RUST_VERSION}-trixie AS builder

RUN apt-get update \
  && apt-get install -y --no-install-recommends cmake ninja-build perl \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /src

COPY rust-server/ ./rust-server/
COPY shared/ ./shared/

WORKDIR /src/rust-server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/src/rust-server/target \
  native_target="$(rustc -vV | sed -n 's/^host: //p')" \
  && test -n "$native_target" \
  && cargo build --locked --release --bin csqtt --target "$native_target" \
  && install -Dm0755 "target/$native_target/release/csqtt" /out/csqtt

FROM debian:trixie-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    iproute2 \
    iptables \
    libgcc-s1 \
    libstdc++6 \
    procps \
  && rm -rf /var/lib/apt/lists/*

LABEL org.opencontainers.image.source="https://github.com/XXcipherX/csqtt-server" \
  org.opencontainers.image.description="CSQTT TURN/RTP tunnel server"

COPY --from=builder /out/csqtt /usr/local/bin/csqtt
COPY LICENSE /usr/share/licenses/csqtt/LICENSE

VOLUME ["/etc/csqtt"]
EXPOSE 46000/udp 46002/tcp

STOPSIGNAL SIGTERM
ENTRYPOINT ["/usr/local/bin/csqtt"]
