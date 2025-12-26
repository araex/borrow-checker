# BorrowChecker
Share expenses with friends to keep track of who owes who. Data of an expense sharing group is stored in a user managed git repository. `BorrowChecker` is a client only web app that uses a built in git client to append changes to the ledger on behalf of the user. 

# Development
## Dependencies
- wasm toolchain: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/): `cargo install --locked trunk`
- **Apple Silicon only** requires manual install of wasm-bindgen: `cargo install --locked wasm-bindgen-cli`
- [`leptosfmt`](https://github.com/bram209/leptosfmt): `cargo install leptosfmt`
- clippy: `rustup component add clippy`

## Auto-Reloading Dev Server
`trunk serve --port 42069 --open`

## Deploying
`trunk build --release`

## Links
- [Leptos Docs]: https://book.leptos.dev
- [Leptops Deploy Docs]: https://book.leptos.dev/deployment/csr.html
- [Trunk Docs]: https://trunkrs.dev/assets/
