# Publishing CytoScnPy MCP Server

## Prerequisites

1.  **Crates.io Account**: You need an account on [crates.io](https://crates.io/).
2.  **API Token**: Run `cargo login <your-token>` locally.

## Step 1: Serialize Workspace Publishing

Since `cytoscnpy-mcp` depends on `cytoscnpy`, they must be published in an order where the dependency is available.

1.  **Publish Core Library First**
    navigate to root:
    ```bash
    cargo publish -p cytoscnpy
    ```
    *Note: If this is the first time publishing `cytoscnpy` as a crate, ensure the name is available on crates.io.*

2.  **Publish MCP Server**
    Once the core library is successfully published (or if you are just publishing the binary and `cytoscnpy` is already up to date on crates.io):
    ```bash
    cargo publish -p cytoscnpy-mcp
    ```

## Step 2: Alternative - GitHub Releases (Binary Distribution)

If you don't want to publish to crates.io and just want to distribute the executable:

1.  **Build Release Binary**:
    ```bash
    cargo build --release -p cytoscnpy-mcp
    ```

2.  **Locate Binary**:
    The binary is at `target/release/cytoscnpy-mcp.exe` (Windows) or `target/release/cytoscnpy-mcp` (Linux/Mac).

3.  **Upload**:
    Create a new Release on GitHub and upload this binary. Users can download it and point their Claude/Cursor config to the downloaded file.

## Step 3: Automated Multi-Platform Release (Recommended)

I have set up a GitHub Actions workflow `.github/workflows/mcp-release.yml` that will automatically build binaries for Windows (x64), Linux (x64), and macOS (x64/ARM64) when you push a version tag.

### How to Release
1.  **Tag the commit**:
    ```bash
    git tag v1.0.0
    git push origin v1.0.0
    ```
2.  **Wait**: GitHub Actions will run, build the binaries, and create a Release named `v1.0.0` with the artifacts attached.

### How Users Install
Users can run the provided scripts to install the correct binary for their system:

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.ps1 | iex
```

## Step 4: MCP Registry (Optional)

You can submit your server to the [MCP Servers Directory](https://github.com/modelcontextprotocol/servers) by making a Pull Request to their repository, listing your server under community tools.
