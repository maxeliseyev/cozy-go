# Browser and Node.js bridge

The bridge compiles the same `cozy-go` rules into a WASM module with no imports. Both runtimes
load the embedded module synchronously. Each call supplies the entire position and repetition
history; the instance holds input/output buffers and allocator state only.

```sh
cargo generate-lockfile
node scripts/build-web.mjs
```

Copy `bindings/web/pkg/` to the consuming TypeScript application. It contains the bridge,
embedded WASM bytes, and a manifest with hashes of the WASM and source files. The application
does not need Rust installed to build or run the checked-in artifact. To update the rules,
rebuild and replace the whole directory together, then run the application's rule and archive
tests. All indices in the bridge use the library's lower-left coordinate system.

The bridge provides placement/pass, positional hash, group/liberties, and Chinese area with
territory ownership. Scoring agreements, resignation, archives, and networking remain the
application's responsibility. Values cross the bridge through bounded buffers and fixed-width
little-endian integers; no JSON, floating-point scoring, or runtime randomness is used in WASM.
