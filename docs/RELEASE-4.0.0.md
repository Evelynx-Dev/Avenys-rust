# Avenys 4.0.0 release notes

**Release status: breaking release.** Avenys is now the compiler; Owl 1.0.0
owns projects, dependencies, registries, lockfiles and project repair. The
version jump is intentional: the compiler has stopped wearing a project
manager costume.

## Baseline and migration timeline

These notes compare the release with the last upstream baseline, `v3.18.0`,
and summarize the intermediate 3.24.x development milestones. Those milestones
were migration steps, not compatibility promises:

- `v3.18.0`: upstream baseline.
- `v3.24.x`: MIR drop/concat work, demand-driven runtime linking, freestanding
  experiments, explicit cache/output paths, warning controls and first-stage
  Owl/Avenys separation.
- `v4.0.0`: the separation is now the contract, with `.mr` support, PAL and
  runtime integration coverage, hierarchical networking APIs and CI execution.

## What changed

- Project discovery, package resolution, registries, installation and lockfiles
  moved to Owl. Avenys receives explicit input and Owl-generated `--config`.
- The compiler CLI is centered on `build`, `run`, `test` and `debug`.
- `artifact = "bin" | "static" | "shared"` replaces `--libt`.
- Runtime tier, target, cache, output, link directories and library roots are
  explicit rather than guessed from a project directory.
- `.mire` and `.mr` are accepted by source discovery, local loads, exports,
  macro files and the test harness.
- LLVM object generation, `llvm-ar` archives and `ld.lld` shared linking are
  first-class paths. Clang remains a target-aware build helper, not a runtime
  dependency of compiled Mire programs.
- PAL v4, process integration, crypto, TCP and UDP support were expanded.
  Listener datagrams now support send/receive and have a native loopback test.
- libsodium-backed hashing, secure randomness and Ed25519 paths are covered by
  Mire integration tests. Base64 decoding now rejects malformed RFC 4648 data.
- The modular Mire suite records metrics only with `--log` and runs in CI in
  addition to Rust tests. Warnings remain opt-in and can be promoted or denied.
- Unused SDL and desktop packages were removed from compiler CI. The Docker
  fixture remains as an internal reproducible library/runtime test image, not a
  production dependency.

## Compatibility changes

1. Avenys no longer auto-resolves projects or registries. Use Owl or provide
   compiler inputs and configuration explicitly.
2. Dependency management commands must be run through Owl.
3. `--libt` is removed; use `--artifact`.
4. Build tooling must pass `--config`, `--lib-dir`, `--cache-dir`, `--output`
   and related flags when required.
5. Flat network constructors were replaced with explicit namespaces:
   `net::socket::connect::tcp/udp` and `net::listener::bind::tcp/udp`, with
   datagram operations under `net::listener::send/recv`.
6. The PAL contract is structured and tested, but the broader Mire ABI is not
   yet frozen for arbitrary third-party consumers. Pin compiler, runtime and
   libraries together for production artifacts.
7. Historical benchmark and syntax-helper scripts were removed from the
   compiler workflow; benchmark projects or CI now own performance checks.

## Migration example

```text
3.x:  mire -> discovers project -> resolves packages -> builds
4.0:  owl  -> validates owl.toml/lockfile -> emits config -> mire builds
```

```bash
mire build code/main.mr \
  --config .mire-config.toml \
  --runtime minimal \
  --artifact bin \
  --cache-dir bin/.cache
```

Managed projects should use `owl build`, `owl run` or `owl test`; Owl supplies
resolved libraries and compiler configuration. One tool owns each failure,
which is considerably less telephone and considerably more traceability.

## Verification

The release gate covers Rust compiler tests, modular Mire tests, native PAL
smoke tests, crypto/proc/net integration and a Docker CI environment. A local
sandbox may deny network sockets; that is reported as an environment limit,
not converted into a fake `return true` victory lap. Even `dasu` deserves a
better joke than that.
