# ── Build ─────────────────────────────────────────────────────────────────────
# Edition 2024 exige Rust >= 1.85. `git` é necessário porque as crates pharos-*
# são dependências git (Cargo.toml) e `.cargo/config.toml` usa git-fetch-with-cli.
FROM rust:1-bookworm AS builder

WORKDIR /app

# Camada de dependências: copia manifestos primeiro para aproveitar o cache do Docker
# enquanto só o código-fonte muda.
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
# Stub mínimo para compilar as dependências sem o código real.
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release --bin finledger || true
RUN rm -rf src

# Código real.
COPY src ./src
# `touch` invalida o cache do artefato final para recompilar com o código novo.
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --locked --bin finledger

# ── Runtime ───────────────────────────────────────────────────────────────────
# Imagem enxuta: TLS do Postgres é rustls e o hash de senha é Argon2 puro-Rust,
# então só precisamos de ca-certificates.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Usuário sem privilégios.
RUN useradd --system --uid 10001 finledger
USER finledger

COPY --from=builder /app/target/release/finledger /usr/local/bin/finledger

EXPOSE 3000
CMD ["finledger"]
