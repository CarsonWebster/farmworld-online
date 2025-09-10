# Agent Guidelines for FarmWorld Online

## Build/Test Commands
- **Godot Client**: Run from Godot editor or `godot --path . --export "Windows Desktop" build/farmworld.exe`
- **Rust Server**: `cargo build` (debug), `cargo build --release` (optimized)
- **Run Server**: `cargo run` (from farmworld-online-server/)
- **Test Server**: `cargo test` (runs all Rust tests)
- **Single Test**: `cargo test test_name` (replace test_name with specific test function)
- **Lint Rust**: `cargo clippy` (linting), `cargo fmt` (formatting)

## Code Style Guidelines

### GDScript (Godot)
- **Naming**: PascalCase for classes/nodes, snake_case for variables/functions/signals
- **Type Hints**: Always use type hints (e.g., `var speed: float`, `func _ready() -> void`)
- **Exports**: Use `@export` for inspector-exposed variables
- **Signals**: Define signals with `signal` keyword, emit with `emit_signal()`
- **Error Handling**: Use `assert()` for debug checks, handle nulls explicitly

### Rust (Server)
- **Formatting**: Use `cargo fmt` (rustfmt) for consistent formatting
- **Linting**: Use `cargo clippy` for code quality checks
- **Imports**: Group std imports first, then external crates, then local modules
- **Error Handling**: Use `Result<T, E>` and `?` operator, avoid unwrap() in production
- **Naming**: snake_case for functions/variables, PascalCase for types/structs
- **Documentation**: Use `///` for public API documentation
- **Async**: Use tokio for async operations, avoid blocking operations in async contexts

### General
- **UTF-8**: All files use UTF-8 encoding
- **No Comments**: Avoid unnecessary comments, code should be self-documenting
- **Security**: Never log sensitive data, validate all inputs, use secure defaults