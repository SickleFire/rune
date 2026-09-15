# Install Rune CLI

### Automated Installation (macOS & Linux)
You can install Rune automatically using the installer script included in the release archive or via curl:
```bash
curl -sSL https://raw.githubusercontent.com/SickleFire/rune/main/install.sh | bash
```

### Custom Installation Directory
By default, the script installs `rune` into `$HOME/.local/bin`. You can customize the install directory by setting the `INSTALL_DIR` environment variable:
```bash
INSTALL_DIR=/usr/local/bin curl -sSL https://raw.githubusercontent.com/SickleFire/rune/main/install.sh | sudo bash
```

### Manual Installation
1. Go to the [Releases](https://github.com/SickleFire/rune/releases) page.
2. Download the archive appropriate for your OS and architecture (`-linux-gnu.tar.gz`, `-apple-darwin.tar.gz`, or `-windows-msvc.zip`).
3. Extract the archive.
4. (Optional on Unix) Run the included `install.sh` script from within the extracted folder, or manually place the `rune` binary in a directory included in your system's `PATH`.
5. Verify installation with `rune --version`.
