#!/usr/bin/env fish
cd $(status dirname)

cd backend/roborally-frontend-wasm
wasm-pack build --target web --weak-refs

cd ../../roborally-frontend
yarn run build
cp -rT dist ../backend/www

cd ../backend
env PORT=3000 cargo r -p roborally-server
