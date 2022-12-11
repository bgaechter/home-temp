FROM rust:1-alpine

WORKDIR /usr/src/myapp
COPY . .
RUN apk add --no-cache \
        ca-certificates \
        gcc \
		openssl-dev \
		musl-dev &&\
        cargo install --path . 

CMD ["home-temp"]
