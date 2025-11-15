# Build for the web

1. Make sure you have both `Rust` and `npm` (which should include `npx`) installed.

2. Run all commands from the `web/` directory:

```console
npm install
npx wasm-pack build ".." --target web --out-name web --out-dir ./web/pkg
npm run serve
```

3. Open `http://localhost:8080` in a browser

Source: https://github.com/asny/three-d/blob/master/web/README.md