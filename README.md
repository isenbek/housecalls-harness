# housecalls-harness

The local backend for the free trades tools at
[housecalls.bradley.io](https://housecalls.bradley.io/housecalls): one
headless Rust engine that owns the schema, the seed-phrase crypto, and the
zero-knowledge backup envelope, so the apps can run entirely on the phone.

Built in the open, like the hunt it belongs to. The plan and its reasoning:
[local-harness-plan](https://housecalls.bradley.io/housecalls/docs/local-harness-plan).

## The promises this code exists to keep

- **The database lives on the device.** DuckDB runs in the browser
  (duckdb-wasm); this engine emits its schema, migrations, and statements.
- **Backup is a twelve-word code sequence.** BIP-39 words, Argon2id to a
  key that never leaves the process, AES-256-GCM over the serialized
  database. No account, no email, no password.
- **The server holds ciphertext.** The stash id derives from the words
  (independently of the key); the stash body is an opaque envelope. The
  entire database of stashes is other people's ciphertext.
- **Same SQL everywhere.** The browser executes engine SQL through
  duckdb-wasm prepared statements; native tests execute the same SQL
  through the DuckDB CLI. One brain, injected executors.

## Layout

```
shell/             the P4 Dioxus shell: the quote pad native, riding the
                   engine directly (money, words, envelope; no JS).
                   Desktop runs, web compiles, and ANDROID BUILDS: a
                   10.6MB release APK (5.3MB native lib, arm64), P4.2
                   2026-09-07. dx build --platform android with
                   ANDROID_HOME/NDK_HOME/JAVA_HOME set; iOS awaits a Mac.
engine/            the crate (also builds to wasm via wasm-pack)
  src/seed.rs      12 words: generate, normalize, to seed bytes
  src/keys.rs      Argon2id enc key + independent SHA-256 stash id
  src/vault.rs     the HCV1 envelope: AES-GCM encrypt/decrypt
  src/store.rs     schema, one-way migrations, statement templates
  src/sync.rs      stash version seatbelt + integrity hash
  src/sqlrender.rs safe literal rendering for executors without params
  src/wasm_api.rs  the wasm-bindgen surface (keys never cross it)
  tests/           the DuckDB-CLI executor: engine SQL on real DuckDB
pkg/               wasm-pack output (engine is ~270KB before gzip)
www/               browser proof page (engine + duckdb-wasm end to end)
```

## Status

LIVE (2026-09-06): the engine powers the History & backup panel on the
quote and change-order pads at housecalls.bradley.io, proven end to end in
production: backup, full device wipe, restore by the twelve words alone.
Snapshots ride DuckDB's EXPORT/IMPORT DATABASE (parquet) packed into one
blob; the wasm file-layer paths that did not survive contact are recorded
in the bradley.io integration (lib/housecalls/harness-client.ts).

## Build and test

```
cargo test                                   # 23 tests incl. real-DuckDB executor + shell wallet loop
cargo build -p hc-shell                      # native desktop shell (webkit2gtk on Linux)
wasm-pack build engine --target web --out-dir ../pkg --release
```

## License

None yet, on purpose: source-visible, all rights reserved, revisited with
counsel when the House Calls kit's licensing is decided (the reasoning is
in the public template-repo plan). Reading and learning from it is why it
is public; ask before shipping it.
