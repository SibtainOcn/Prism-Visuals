<!--
PURPOSE: Windows installer creation guide using WiX v5
MAINTAINER: SibtainOcn
LAST UPDATED: 2026-01-05
-->

# WiX v5 Installer Guide

## Overview

This document explains how to build the Windows Installer (MSI) for Prism Visuals using WiX Toolset v5.

---

## Prerequisites

| Requirement | Version |
|-------------|---------|
| WiX Toolset | v5+ |
| .NET SDK | 6.0+ |
| Rust | Latest stable |

### Install WiX v5

```powershell
dotnet tool install --global wix
```

---

## Quick Build

```powershell
# 1. Build the application
cargo build --release

# 2. Add UI extension (one-time)
wix extension add WixToolset.UI.wixext

# 3. Build the installer
wix build -arch x64 -ext WixToolset.UI.wixext -out installer_output\visuals-1.2.5-x64.msi main.wxs




cargo build --release; wix build -arch x64 -ext WixToolset.UI.wixext -out installer_output\visuals-1.2.5-x64.msi main.wxs



```

---

## Common Errors

### WIX0094: Identifier 'WixUI:WixUI_Minimal' not found

**Cause:** UI extension not installed or not referenced.

**Fix:**
```powershell
wix extension add WixToolset.UI.wixext
wix extension list
```

---

## WiX v5 Syntax

### Namespace Declaration

```xml
<Wix xmlns='http://wixtoolset.org/schemas/v4/wxs'
     xmlns:ui='http://wixtoolset.org/schemas/v4/wxs/ui'>
```

### UI Reference

| WiX v3 (Old) | WiX v5 (New) |
|--------------|--------------|
| `<UIRef Id='WixUI_Minimal' />` | `<ui:WixUI Id="WixUI_Minimal" />` |

---

## Build Process

### Step 1: Build Application

```powershell
cargo build --release
```

### Step 2: Verify Files

| File | Location |
|------|----------|
| Executable | `target\release\visuals.exe` |
| WiX Source | `main.wxs` |
| License | `License.rtf` |

### Step 3: Build Installer

```powershell
mkdir installer_output
wix build -arch x64 -ext WixToolset.UI.wixext -out installer_output\visuals-1.2.3-x64.msi main.wxs
```

### Output

| File | Description |
|------|-------------|
| `visuals-x.x.x-x64.msi` | Installer package |
| `visuals-x.x.x-x64.wixpdb` | Debug symbols (optional) |

---

## Installation Details

### What Gets Installed

| Item | Location |
|------|----------|
| Executable | `C:\Program Files\Prism Visuals\` |
| PATH Entry | System PATH (requires admin) |
| Shortcut | Start Menu > Prism Visuals |
| Registry | `HKCU\Software\SibtainOcn\Prism Visuals` |

### Uninstallation

**Methods:**

1. Settings > Apps > Apps & features > Prism Visuals > Uninstall
2. Control Panel > Programs > Uninstall
3. Command line: `msiexec /x visuals-x.x.x-x64.msi`

**Removed:**
+ Installation directory
+ PATH entry
+ Start Menu shortcuts
+ Registry entries

**Preserved:**
+ User wallpapers (`%USERPROFILE%\Pictures\Prism Visuals\`)
+ Configuration (`%APPDATA%\Prism Visuals\`)

---

## Customization

### License Agreement

1. Create `License.rtf` in project root
2. Add to `main.wxs`:
   ```xml
   <WixVariable Id="WixUILicenseRtf" Value="License.rtf" />
   ```
3. Rebuild

### Banner Images

| Image | Size | Format |
|-------|------|--------|
| Banner | 493 x 58 px | BMP |
| Dialog | 493 x 312 px | BMP |

```xml
<WixVariable Id="WixUIBannerBmp" Value="banner.bmp" />
<WixVariable Id="WixUIDialogBmp" Value="dialog.bmp" />
```

### UI Templates

| Template | Description |
|----------|-------------|
| `WixUI_Minimal` | License only |
| `WixUI_InstallDir` | Custom install path |
| `WixUI_FeatureTree` | Feature selection |
| `WixUI_Advanced` | Full customization |

---

## GUIDs

| Purpose | Example |
|---------|---------|
| UpgradeCode | `12345678-1234-1234-1234-123456789012` |
| MainExecutable | `11111111-1111-1111-1111-111111111111` |
| PATH Component | `87654321-4321-4321-4321-210987654321` |

**Generate new GUID:**
```powershell
[guid]::NewGuid()
```

---

## Upgrades

The `<MajorUpgrade>` element handles automatic upgrades:

1. Update version in `main.wxs`
2. Keep same `UpgradeCode`
3. Rebuild installer
4. Users install new version over old

---

## Command Reference

```powershell
# Install WiX
dotnet tool install --global wix

# Add UI extension
wix extension add WixToolset.UI.wixext

# List extensions
wix extension list

# Build installer
wix build -arch x64 -ext WixToolset.UI.wixext -out installer_output\visuals-1.2.3-x64.msi main.wxs

# Build with verbose logging
wix build -arch x64 -ext WixToolset.UI.wixext -out installer_output\visuals-1.2.3-x64.msi main.wxs -v

# Test install (admin)
msiexec /i installer_output\visuals-1.2.3-x64.msi

# Test uninstall
msiexec /x visuals-1.2.3-x64.msi

# Silent install
msiexec /i visuals-1.2.3-x64.msi /qn

# Silent uninstall
msiexec /x visuals-1.2.3-x64.msi /qn
```

---

## Resources

+ [WiX v5 Documentation](https://wixtoolset.org/docs/intro/)
+ [UI Extension Reference](https://wixtoolset.org/docs/schema/ui/)

---

## Summary

| Item | Value |
|------|-------|
| Product | Prism Visuals |
| Publisher | SibtainOcn |
| Executable | visuals.exe |
| Command | visuals |
| Install Path | `C:\Program Files\Prism Visuals\` |

**Key Points:**
+ Use `xmlns:ui` namespace for UI elements
+ Use `<ui:WixUI>` instead of `<UIRef>`
+ Always include `-ext WixToolset.UI.wixext` in build command
+ Uninstaller is created automatically
+ User data is preserved on uninstall
