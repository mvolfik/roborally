FROM rust:1 as rust-wasm-builder
WORKDIR /builder
RUN rustup override set nightly-2022-03-26
RUN rustup target add --toolchain nightly-2022-03-26 wasm32-unknown-unknown

RUN cargo install --git https://github.com/mvolfik/wasm-pack --branch merged-1119-and-937

RUN mkdir -p ./backend/roborally-structs/src \
          ./backend/roborally-frontend-wasm/src \
 && touch ./backend/roborally-structs/src/lib.rs \
 && touch ./backend/roborally-frontend-wasm/src/lib.rs \

COPY ./backend/Cargo.toml ./backend/Cargo.lock ./backend/

COPY ./backend/roborally-structs/Cargo.toml ./backend/roborally-structs/
RUN cd backend && cargo build --release --frozen -p roborally-structs --features client --target wasm32-unknown-unknown

COPY ./backend/roborally-frontend-wasm/Cargo.toml ./backend/roborally-frontend-wasm/
RUN cd backend && cargo build --release --frozen -p roborally-frontend-wasm --target wasm32-unknown-unknown

COPY ./backend/roborally-structs/src ./backend/roborally-structs/src
COPY ./backend/roborally-frontend-wasm/src ./backend/roborally-frontend-wasm/src
RUN cd backend && wasm-pack --release --target web --weak-refs



FROM node:16 as node-builder
WORKDIR /builder

COPY ./roborally-frontend/package.json ./roborally-frontend/yarn.lock ./roborally-frontend/
RUN cd roborally-frontend && yarn install

COPY --from=rust-wasm-builder /builder/backend/roborally-frontend-wasm/pkg ./backend/roborally-frontend-wasm/pkg
COPY ./roborally-frontend/public ./roborally-frontend/public
COPY ./roborally-frontend/src ./roborally-frontend/src
COPY ./roborally-frontend/svelte.config.js ./roborally-frontend/vite.config.ts ./roborally-frontend/index.html ./roborally-frontend/
RUN cd roborally-frontend && ln -s ../backend/roborally-frontend-wasm/pkg/ frontend-wasm && yarn run build



FROM rust:1 as rust-server-builder
WORKDIR /builder
RUN rustup override set nightly-2022-03-26

RUN mkdir -p ./backend/roborally-structs/src \
 && touch ./backend/roborally-structs/src/lib.rs \
 && echo 'fn main() {}' > ./backend/roborally-server/src/main.rs

COPY ./backend/Cargo.toml ./backend/Cargo.lock ./backend/

COPY ./backend/roborally-structs/Cargo.toml ./backend/roborally-structs/
RUN cd backend && cargo build --release --frozen -p roborally-structs --features server

COPY ./backend/roborally-server/Cargo.toml ./backend/roborally-server/
RUN cd backend && cargo build --release --frozen -p roborally-server

COPY ./backend/roborally-structs/src ./backend/roborally-structs/src
COPY ./backend/roborally-server/src ./backend/roborally-server/src
RUN cd backend && cargo build --release --frozen -p roborally-server


FROM debian:bullseye-slim
WORKDIR /app

COPY --from=node-builder /builder/roborally-frontend/dist ./roborally-frontend/dist
COPY --from=rust-server-builder /builder/backend/target/release/roborally-server ./backend

WORKDIR /app/backend
CMD ["./backend"]
