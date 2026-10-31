# mumquiz as a service. Build, then run with the config in a volume and the certificate on a data
# volume so the bot keeps its identity.
#
#   docker build -t mumquiz .
#   docker run -d --name mumquiz -v "$PWD/bot.toml:/config/bot.toml:ro" -v mumquiz-data:/data mumquiz
#
# The secret comes from the environment instead of the file:
#   docker run -d -e SERVER_PASSWORD=... -e RUST_LOG=info ...

FROM rust:1.90-alpine AS build
WORKDIR /src
# The bundled SQLite needs a C toolchain here, not at runtime.
RUN apk add --no-cache build-base
COPY . .
RUN cargo install --path . --root /out --locked

FROM alpine:3.20
RUN addgroup -g 10001 -S mumquiz && adduser -u 10001 -S mumquiz -G mumquiz
COPY --from=build /out/bin/mumquiz /usr/local/bin/mumquiz
USER 10001
ENV MUMQUIZ_CONFIG=/config/bot.toml MUMQUIZ_DATA=/data
VOLUME ["/config", "/data"]
HEALTHCHECK CMD true
CMD ["mumquiz"]
