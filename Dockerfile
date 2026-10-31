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

FROM scratch
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /out/khalid /khalid
COPY --from=build --chmod=1777 /out/tmp /tmp
COPY --from=build --chown=65532:65532 /out/site /site
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
