# syntax=docker/dockerfile:1
FROM rust:1.82 AS env-build

# set work directory and copy source
WORKDIR /srv
COPY . /srv/

# build binary and calculate checksum
RUN cargo build --release \
  && cp target/release/luavisors /srv/luavisors-glibc-amd64 \
  && sha256sum luavisors-glibc-amd64 > SHA256SUMS

FROM gcr.io/distroless/cc:nonroot AS env-deploy

# copy binary into distroless container
COPY --from=env-build /srv/luavisors-glibc-amd64 /bin/luavisors
COPY lua/ /app/

USER nonroot

CMD [ "luavisors", "/app/advanced.lua" ]
