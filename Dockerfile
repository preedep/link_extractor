FROM alpine:3.19
RUN mkdir /app
WORKDIR /app
ADD ./target/x86_64-unknown-linux-musl/release/link_extractor /app/link_extractor
ENTRYPOINT ["/app/link_extractor"]


