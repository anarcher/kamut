# KAMUT PROJECT GUIDE

## Commands
- Build: `cargo build`
- Run: `cargo run`
- Test all: `cargo test`
- Test single: `cargo test test_name`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Check format: `cargo fmt -- --check`
- Documentation: `cargo doc --open`

## Code Style Guidelines
- Follow Rust standard formatting with `cargo fmt`
- Use `Result<T, E>` for error handling with appropriate error types
- Organize imports: std first, then external crates, then local modules
- Use snake_case for variables and functions, CamelCase for types
- Document public APIs with doc comments (`///`)
- Prefer pattern matching over if-let chains where appropriate
- Use strong typing with minimal use of `impl Trait` outside of function returns
- Follow Rust's ownership model; prefer borrowing over cloning when possible