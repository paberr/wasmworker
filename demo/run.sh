rm -rf pkg
wasm-pack build --target web --no-opt
python3 -m http.server 8000
