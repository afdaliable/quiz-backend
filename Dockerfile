FROM rust:1.78-bookworm AS base_deps
# ini untuk base dependency
RUN apt update -y && apt upgrade -y
RUN apt install build-essential -y
RUN apt install lld clang -y
# Install OpenSSL dev libraries
RUN apt-get install -y pkg-config libssl-dev

FROM base_deps AS build
ENV USER=app
ENV UID=10001
RUN adduser --disabled-password --gecos "" --home "/nonexistent" --shell "/sbin/nologin" --no-create-home --uid "${UID}" "${USER}"
WORKDIR /app
COPY . .
# Build with static linking for OpenSSL
ENV OPENSSL_STATIC=yes
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include/openssl
RUN cargo b -r

FROM gcr.io/distroless/cc-debian12 AS deployment
# Copy SSL certificates
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group
WORKDIR /app
COPY --from=build /app/target/release/quiz-backend ./
USER app
EXPOSE 8787
CMD ["./quiz-backend"]