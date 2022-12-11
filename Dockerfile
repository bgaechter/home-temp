FROM rust:1-slim as builder

WORKDIR /usr/src/myapp
COPY . .

RUN apt-get update -y &&\
		apt-get upgrade -y &&\
		apt-get install -y --no-install-recommends ca-certificates pkg-config gcc libssl-dev libc6-dev
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/home-temp /usr/local/bin/home-temp
CMD ["home-temp"]
