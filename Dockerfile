FROM rust:1-slim

WORKDIR /usr/src/myapp
COPY . .

RUN apt-get update -y &&\
		apt-get upgrade -y &&\
		apt-get install -y --no-install-recommends ca-certificates pkg-config gcc libssl-dev libc6-dev

RUN cargo install --path .

CMD ["home-temp"]
