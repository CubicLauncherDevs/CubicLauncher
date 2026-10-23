# syntax=docker/dockerfile:1

FROM oven/bun:1 AS frontend

WORKDIR /src

COPY package.json bun.lock ./
RUN bun install --frozen-lockfile

COPY . .
RUN bun run build

FROM ubuntu:22.04 AS builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        libayatana-appindicator3-dev \
        libgtk-3-dev \
        librsvg2-dev \
        libssl-dev \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /src

COPY --from=frontend /src/node_modules ./node_modules
COPY . .
COPY --from=frontend /src/build ./build

RUN cargo build --release --locked -p cubiclauncher

FROM ubuntu:22.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/data

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        dbus \
        fonts-dejavu-core \
        glib-networking \
        gsettings-desktop-schemas \
        libasound2 \
        libayatana-appindicator3-1 \
        libdrm2 \
        libegl1 \
        libgbm1 \
        libgl1 \
        libgl1-mesa-dri \
        libgles2 \
        libgtk-3-0 \
        libpulse0 \
        librsvg2-2 \
        libssl3 \
        libsoup-3.0-0 \
        libudev1 \
        libwebkit2gtk-4.1-0 \
        libx11-6 \
        libxcursor1 \
        libxdo3 \
        libxi6 \
        libxinerama1 \
        libxrandr2 \
        libxxf86vm1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 --shell /usr/sbin/nologin cubic \
    && mkdir -p /data \
    && chown cubic:cubic /data \
    && chmod 1777 /data

COPY --from=builder /src/target/release/cubiclauncher /usr/local/bin/cubiclauncher

USER cubic
WORKDIR /data

ENTRYPOINT ["dbus-run-session", "--", "cubiclauncher"]
