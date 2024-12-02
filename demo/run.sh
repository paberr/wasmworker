rm -rf pkg
wasm-pack build --target bundler --no-opt
python3 -m http.server 8000
