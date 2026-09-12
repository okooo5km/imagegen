# Native CLI portability and Rust practices

Verified primary-source research on 2026-09-12 informed the Rust v2 migration.

- Rust 2024 edition, explicit MSRV 1.88, `Cargo.lock`, locked CI/release builds, fmt and warning-free Clippy. No project unsafe code; dependencies may contain their own audited platform/crypto internals.
- `clap` provides typed, consistent command parsing/help. `serde` rejects unknown config/request fields. TOML defaults are separate from the API key.
- `reqwest` blocking clients reuse connections; `rustls` avoids OpenSSL deployment. Authenticated requests do not follow redirects; transport retries are disabled. Downloads use a separate unauthenticated client.
- `PathBuf`, UTF-8 text with BOM/CRLF handling, Windows native console output, `rpassword` hidden input and atomic `tempfile` writes avoid shell-specific application logic.
- Linux musl packages are static; Windows uses `crt-static`; macOS targets use the system libraries supplied with supported macOS. Runtime installation needs no language toolchain.
- Six native runner combinations test actual executable behavior rather than inferring Windows/ARM support from a Mac build. Installers verify hashes before extraction/replacement and are themselves tested with paths containing spaces and corrupt archives.
- CLI flags override persistent defaults. Exact dimensions and aspect-ratio/resolution mode have explicit precedence; edits preserve reference geometry unless the user opts into changing it.
- Receipts preserve task IDs and output paths, not secrets/prompts. Fetch recovers tasks without repeating paid submissions. Polling and HTTP operations are bounded.
- Tests use isolated temporary config roots and local HTTP servers. Paid smoke tests are separate and authorized; they verify service integration, not a permanent uptime guarantee.

Distribution uses a small explicit GitHub Actions workflow and shell/PowerShell installers, after evaluating cargo-dist's installer approach. This project's six archives, checksum verification and installation tests do not require a separate packaging toolchain.

Primary sources:

- [Rust 2024 project creation](https://doc.rust-lang.org/edition-guide/editions/creating-a-new-project.html)
- [Rust-version-aware resolver](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)
- [Cargo.lock guidance](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html)
- [reqwest clients, TLS and features](https://docs.rs/reqwest/latest/reqwest/)
- [cargo-dist shell installers](https://axodotdev.github.io/cargo-dist/book/installers/shell.html)
- [GitHub native runner platforms](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [Agent Skills specification](https://agentskills.io/specification)

The local skill-creator validator predates the optional `compatibility` frontmatter key, so runtime requirements are expressed in the skill body and setup reference. The skill remains self-contained documentation for the installed native CLI, with no references to removed Python scripts.
