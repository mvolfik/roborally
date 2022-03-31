FROM docker.io/library/rust:1-bullseye as rust-wasm-builder
WORKDIR /builder
RUN rustup override set nightly-2022-03-26
RUN rustup target add --toolchain nightly-2022-03-26 wasm32-unknown-unknown

RUN cargo install --git https://github.com/mvolfik/wasm-pack --branch merged-1119-and-937

RUN mkdir -p ./backend/roborally-structs/src \
             ./backend/roborally-frontend-wasm/src \
             ./backend/roborally-server/src \
 && touch ./backend/roborally-structs/src/lib.rs \
 && touch ./backend/roborally-frontend-wasm/src/lib.rs \
 && echo 'fn main() {}' > ./backend/roborally-server/src/main.rs

COPY ./backend/Cargo.toml ./backend/Cargo.lock ./backend/

COPY ./backend/roborally-structs/Cargo.toml ./backend/roborally-structs/
COPY ./backend/roborally-frontend-wasm/Cargo.toml ./backend/roborally-frontend-wasm/
COPY ./backend/roborally-server/Cargo.toml ./backend/roborally-server/
RUN cd backend && cargo build --release --locked -p roborally-frontend-wasm --target wasm32-unknown-unknown

# Touch entry file after each copy - cargo apparently caches by modification time,
# and real source files are most likely order than the above created 'dummy' ones
COPY ./backend/roborally-structs/src ./backend/roborally-structs/src
RUN touch ./backend/roborally-structs/src/lib.rs
COPY ./backend/roborally-frontend-wasm/src ./backend/roborally-frontend-wasm/src
RUN touch ./backend/roborally-frontend-wasm/src/lib.rs
RUN cd backend/roborally-frontend-wasm && wasm-pack build --release --target web --weak-refs



FROM docker.io/library/node:16 as node-builder
WORKDIR /builder

COPY ./roborally-frontend/package.json ./roborally-frontend/yarn.lock ./roborally-frontend/
RUN cd roborally-frontend && yarn install

COPY --from=rust-wasm-builder /builder/backend/roborally-frontend-wasm/pkg ./backend/roborally-frontend-wasm/pkg
COPY ./roborally-frontend/public ./roborally-frontend/public
COPY ./roborally-frontend/src ./roborally-frontend/src
COPY ./roborally-frontend/svelte.config.js ./roborally-frontend/vite.config.ts ./roborally-frontend/index.html ./roborally-frontend/
RUN cd roborally-frontend && yarn run build



FROM docker.io/library/rust:1-bullseye as rust-server-builder
WORKDIR /builder
RUN rustup override set nightly-2022-03-26

RUN mkdir -p ./backend/roborally-structs/src \
             ./backend/roborally-frontend-wasm/src \
             ./backend/roborally-server/src \
 && touch ./backend/roborally-structs/src/lib.rs \
 && touch ./backend/roborally-frontend-wasm/src/lib.rs \
 && echo 'fn main() {}' > ./backend/roborally-server/src/main.rs

COPY ./backend/Cargo.toml ./backend/Cargo.lock ./backend/

COPY ./backend/roborally-structs/Cargo.toml ./backend/roborally-structs/
COPY ./backend/roborally-frontend-wasm/Cargo.toml ./backend/roborally-frontend-wasm/
COPY ./backend/roborally-server/Cargo.toml ./backend/roborally-server/
RUN cd backend && cargo build --release --locked -p roborally-server

# Touch entry file after each copy - cargo apparently caches by modification time,
# and real source files are most likely order than the above created 'dummy' ones
COPY ./backend/roborally-structs/src ./backend/roborally-structs/src
RUN touch ./backend/roborally-structs/src/lib.rs
COPY ./backend/roborally-server/src ./backend/roborally-server/src
RUN touch ./backend/roborally-server/src/lib.rs
COPY ./maps ./maps
RUN cd backend && cargo build --release --locked -p roborally-server



FROM docker.io/library/rust:1-bullseye as rust-server-builder-win
WORKDIR /builder
RUN rustup override set nightly-2022-03-26
RUN rustup target add --toolchain nightly-2022-03-26 x86_64-pc-windows-gnu
RUN apt-get update && apt-get install -y gcc-mingw-w64-x86-64-win32 && rm -rf /var/lib/apt/lists/*

RUN mkdir -p ./backend/roborally-structs/src \
             ./backend/roborally-frontend-wasm/src \
             ./backend/roborally-server/src \
 && touch ./backend/roborally-structs/src/lib.rs \
 && touch ./backend/roborally-frontend-wasm/src/lib.rs \
 && echo 'fn main() {}' > ./backend/roborally-server/src/main.rs

COPY ./backend/Cargo.toml ./backend/Cargo.lock ./backend/

COPY ./backend/roborally-structs/Cargo.toml ./backend/roborally-structs/
COPY ./backend/roborally-frontend-wasm/Cargo.toml ./backend/roborally-frontend-wasm/
COPY ./backend/roborally-server/Cargo.toml ./backend/roborally-server/
RUN cd backend && cargo build --release --locked -p roborally-server --target x86_64-pc-windows-gnu

# Touch entry file after each copy - cargo apparently caches by modification time,
# and real source files are most likely order than the above created 'dummy' ones
COPY ./backend/roborally-structs/src ./backend/roborally-structs/src
RUN touch ./backend/roborally-structs/src/lib.rs
COPY ./backend/roborally-server/src ./backend/roborally-server/src
RUN touch ./backend/roborally-server/src/lib.rs
COPY ./maps ./maps
RUN cd backend && cargo build --release --locked -p roborally-server --target x86_64-pc-windows-gnu

FROM debian:bullseye-slim as zipper
WORKDIR /zipper
COPY --from=node-builder /builder/roborally-frontend/dist ./roborally/www
COPY --from=rust-server-builder /builder/backend/target/release/roborally-server ./roborally/
RUN tar -czvf roborally-dist-linux.tar.gz roborally && rm ./roborally/roborally-server
COPY --from=rust-server-builder-win /builder/backend/target/x86_64-pc-windows-gnu/release/roborally-server.exe ./roborally/
RUN tar -czvf roborally-dist-windows.tar.gz roborally && rm ./roborally/roborally-server.exe

FROM debian:bullseye-slim
WORKDIR /app

COPY --from=rust-server-builder /builder/backend/target/release/roborally-server ./
COPY --from=node-builder /builder/roborally-frontend/dist ./www
COPY --from=zipper /zipper/roborally-dist-linux.tar.gz /zipper/roborally-dist-windows.tar.gz ./www/
COPY ./source-code.tar.gz ./www/

CMD ["./roborally-server"]
