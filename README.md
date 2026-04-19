# steam-broker

Simple program that communicates with Steam on behalf of others.

## Compiling

steam-broker is written in pure Rust, so compiling it should be straightforward on any modern platform. You will need [Rust toolchain](https://www.rust-lang.org/tools/install) (with `cargo`) installed.

### Supported Operating Systems

- Fedora / Nobara
- Arch Linux
- Ubuntu / Debian-based
- Windows

---

## Instructions by Operating System

### Fedora / Nobara

```bash
# Install Rust (if not already)
sudo dnf install rust cargo

# Optionally, update to the latest using rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/ilovesodaa/steam-broker.git
cd steam-broker

# Build the program
cargo build --release
```

### Arch Linux

```bash
# Install Rust
sudo pacman -S rust cargo

# Or use rustup for the latest version (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/ilovesodaa/steam-broker.git
cd steam-broker
cargo build --release
```

### Ubuntu (and Debian-based)

```bash
# Install dependencies
sudo apt update
sudo apt install cargo rustc

# Or install with rustup (recommended for latest version)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/ilovesodaa/steam-broker.git
cd steam-broker
cargo build --release
```

### Windows

- Install [rustup](https://rustup.rs/) (download and run the installer).
- Open Command Prompt or PowerShell:

```powershell
git clone https://github.com/ilovesodaa/steam-broker.git
cd steam-broker
cargo build --release
```

_The compiled binary will be in the `target/release` directory._

---

## Notes

- Make sure `cargo` is in your PATH. After installing with rustup, you may need to restart your terminal or run `source $HOME/.cargo/env`.
- For cross-compilation or packaging, refer to the [Rust documentation](https://doc.rust-lang.org/book/ch01-01-installation.html).
- If you encounter issues, ensure you have the latest stable Rust toolchain:  
  `rustup update`
- On some distributions, additional build dependencies like `build-essential` (Ubuntu) or `base-devel` (Arch) may be required for certain crates, but for pure Rust projects this is usually not needed.

---

Enjoy using steam-broker!
