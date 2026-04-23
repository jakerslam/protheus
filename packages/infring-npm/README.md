# infring (npm)

Install globally:

```bash
npm install -g infring
```

or from source:

```bash
cd packages/infring-npm
npm install -g .
```

The package installs an `infring` executable (plus legacy `infring`) backed by the Rust `infring-ops` binary.

```bash
infring --help
infring gateway
```

## Runtime Notes

- Installer first attempts to fetch a prebuilt binary from GitHub Releases.
- If no release binary is available, it falls back to building from source with Cargo (when source files are present).
- When full Infring runtime assets are available, the wrapper routes into `infringctl` command dispatch.
