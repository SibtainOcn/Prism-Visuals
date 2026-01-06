<!--
PURPOSE: Development environment setup
MAINTAINER: SibtainOcn
LAST UPDATED: 2026-01-05
-->

# Required Development Setup

## Prerequisites

| Requirement | Version |
|-------------|---------|
| Rust | Latest stable |
| Visual Studio C++ Build Tools | 2022+ |
| WiX Toolset | v5+ |
| .NET SDK | 6.0+ |

---

## Quick Start

```powershell
# Clone or create project
cd C:\Users\YourName\Desktop
mkdir prism-visuals
cd prism-visuals
cargo init --name visuals

# Build
cargo build --release

# Output
target\release\visuals.exe
```

---

## Project Structure

```
prism-visuals/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs
│   ├── scheduler.rs
│   ├── wallhaven.rs
│   └── pexels.rs
├── target/
│   └── release/
│       └── visuals.exe
├── main.wxs
├── License.rtf
└── docs/
```

---

## Build Commands

```powershell
# Release build
cargo build --release

# Parallel build
cargo build --release -j 8

# Check syntax
cargo check

# Run tests
cargo test
```

---

## Add to PATH

### Option A: Copy to System

```powershell
copy target\release\visuals.exe C:\Windows\System32\
```

### Option B: Add to User PATH

```powershell
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";C:\path\to\target\release",
    "User"
)
```



## Build Optimization

Already configured in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
strip = true
lto = true
```


---

## Create Installer

```powershell
# Install WiX
dotnet tool install --global wix

# Add UI extension
wix extension add WixToolset.UI.wixext

# Build MSI
wix build -arch x64 -ext WixToolset.UI.wixext -out installer_output\visuals-xx.x.xx-x64.msi main.wxs
```




