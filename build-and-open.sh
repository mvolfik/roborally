cargo install --git https://github.com/mvolfik/wasm-pack --branch merged-1119-and-937
cd backend/roborally-frontend-wasm
wasm-pack build --target web --weak-refs
cd ../../roborally-frontend
yarn build
cd ../backend
cargo build --release --package roborally-server
target/release/roborally-server