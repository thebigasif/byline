# syntax=docker/dockerfile:1
#
# Khalid in an empty image: one fully static binary, the certificates it
# needs for HTTPS requests, and nothing else. No shell, no package manager,
# no library to patch. How to run it is in docs/deploy.md.
#
#   docker build -t khalid .
#   docker run -p 8080:8080 -v "$PWD/site:/site" khalid

# Keep these the same as rust-toolchain.toml and the admin job in ci.yml.
ARG RUST_VERSION=1.99.1
ARG NODE_VERSION=22
# Where the binary comes from. `build` compiles it here. The release
# workflow passes `--build-arg BINARY=prebuilt` to use the musl files it
# has already built and checked, one per processor, in dist/<arch>/khalid.
ARG BINARY=build

# The admin first, because the binary embeds what admin/dist holds.
FROM node:${NODE_VERSION}-alpine AS admin
WORKDIR /src/admin
COPY admin/package.json admin/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY admin/ ./
RUN npm run build

# Rust on Alpine builds for musl and links every binary statically.
FROM rust:${RUST_VERSION}-alpine AS build
ARG RUST_VERSION
# The image's own toolchain, without the extra tools rust-toolchain.toml lists.
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}
RUN apk add --no-cache ca-certificates gcc musl-dev
WORKDIR /src
COPY . .
COPY --from=admin /src/admin/dist admin/dist
RUN cargo build --release --locked -p khalid \
    && mkdir -p /out/site /out/tmp \
    && cp target/release/khalid /out/khalid

# The release's own binary, for the processor being built. Docker skips
# this stage unless BINARY names it.
FROM --platform=$BUILDPLATFORM alpine:3.22 AS prebuilt
ARG TARGETARCH
RUN apk add --no-cache ca-certificates && mkdir -p /out/site /out/tmp
COPY --chmod=755 dist/${TARGETARCH}/khalid /out/khalid

FROM ${BINARY} AS binary

FROM scratch
COPY --from=binary /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=binary /out/khalid /khalid
COPY --from=binary --chmod=1777 /out/tmp /tmp
COPY --from=binary --chown=65532:65532 /out/site /site
# The image has no loader and no library, so this runs only when the
# binary is fully static.
RUN ["/khalid", "--version"]
USER 65532:65532
VOLUME /site
EXPOSE 8080
ENTRYPOINT ["/khalid", "--site", "/site"]
# --init makes the site on the first start when /site is empty, at the
# address in KHALID_ADDRESS, as hosts such as Fly.io and Railway need.
CMD ["serve", "--init", "--listen", "0.0.0.0:8080"]
