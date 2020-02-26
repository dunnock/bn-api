FROM rust:1.38.0 as builder

RUN \
    wget https://github.com/openssl/openssl/archive/OpenSSL_1_1_0-stable.zip && \
    unzip OpenSSL_1_1_0-stable.zip && cd openssl-OpenSSL_1_1_0-stable && \
    ./config && make -j $(nproc) && make install
ENV LD_LIBRARY_PATH /usr/local/lib
#RUN apt update && apt install \
#    libpq-dev \
#    musl-tools \
#    openssl \
#    libssl-dev \
#    pkg-config \
#    gcc \
#    make \
#    build-essential \
#    -y

# create a new empty shell project
RUN USER=root cargo new --bin bn-api
WORKDIR /bn-api
# Copy the dependency lists
ADD Cargo.lock ./
ADD Cargo.docker.toml ./Cargo.toml

# this build step will cache our dependencies
RUN cargo build --release
RUN rm src/*.rs

# Add the actual source code
ADD api ./api/
ADD branch_rs ./branch_rs/
ADD db ./db/
ADD facebook ./facebook/
ADD http ./http/
ADD tari-client ./tari-client/
ADD stripe ./stripe/
ADD logging ./logging/
ADD globee ./globee/
ADD embed_dirs_derive ./embed_dirs_derive/
ADD macros ./macros/
ADD customer_io ./customer_io/
ADD cache ./cache/
ADD Cargo.lock Cargo.toml ./

RUN cargo build --release

# Create a base minimal image for adding our executables to
FROM bitnami/minideb:stretch as base
RUN apt update && apt install \
    openssl \
    libpq5 \
    curl \
    -y

# Now create a new image with only the essentials and throw everything else away
FROM base
COPY --from=builder /bn-api/target/release/server /usr/bin/
COPY --from=builder /bn-api/target/release/bndb_cli /usr/bin/
COPY --from=builder /bn-api/target/release/api-cli /usr/bin/

ADD reset-database.sh /usr/bin/

CMD ["server"]
