FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/backend /app/backend
COPY .env /app/.env

ENTRYPOINT ["./backend"]

EXPOSE 3000
