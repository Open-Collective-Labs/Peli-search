# Installation

## Supported Platforms

| Platform | Status |
|----------|--------|
| Linux (x86_64) | Tier 1 |
| Linux (aarch64) | Tier 1 |
| macOS (x86_64) | Tier 1 |
| macOS (aarch64) | Tier 1 |
| Windows (x86_64) | Tier 2 |

## Requirements

- **Rust**: 1.75 or later (if building from source)
- **Memory**: Minimum 256 MB, recommended 1 GB+
- **Disk**: 100 MB for the binary, variable for indexes

## Installation Methods

### Download a Pre-built Binary

```bash
curl -fsSL https://peli.sh/install.sh | sh
```

Or download directly from the [releases page](https://github.com/Open-Collective-Labs/Peli-search/releases).

### Install via Package Managers

**Homebrew (macOS/Linux):**

```bash
brew install peli/tap/peli
```

**Cargo (if you have Rust installed):**

```bash
cargo install pelisearch-server
```

### Build from Source

```bash
git clone https://github.com/Open-Collective-Labs/Peli-search.git
cd Peli-search
cargo build --release
./target/release/pelisearch-server --help
```

## Running Locally

### As an Embedded Library

Add to your `Cargo.toml`:

```toml
[dependencies]
peli-search = "0.1"
```

### As a Standalone Server

```bash
# Start the server with default settings
pelisearch-server

# Start on a custom port
pelisearch-server --port 7700 --data-path ./data
```

The server starts an HTTP API at `http://127.0.0.1:7700` by default.

### Verify Installation

```bash
pelisearch-server --version
```
