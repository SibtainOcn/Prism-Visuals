# Security Policy

## Supported Versions

We actively maintain and provide security updates for the following versions of Prism Visuals:

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | [YES]              |
| < 1.0   | [ NOT ]            |

**Note**: Version numbers are automatically updated with each release. Always use the latest version for the most up-to-date security patches.

---

## Reporting a Vulnerability

We take the security of Prism Visuals seriously. If you discover a security vulnerability, please follow these guidelines for responsible disclosure:

### How to Report

1. **Open a GitHub Issue**: Navigate to the [Issues](../../issues) section of this repository
2. **Use the Security Label**: Tag your issue with the `security` label if available
3. **Provide Details**: Include as much information as possible:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Affected versions
   - Any proof-of-concept code (if applicable)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your vulnerability report within 72 hours
- **Investigation**: We will investigate the issue and provide an estimated timeline for a fix
- **Resolution**: Security patches will be prioritized and released as soon as possible
- **Credit**: You will be credited for the discovery (unless you prefer to remain anonymous)

### Please Do Not

- Publicly disclose the vulnerability before we have had a chance to address it
- Exploit the vulnerability for malicious purposes
- Access or modify data that doesn't belong to you

---

## Security Considerations

### Application Security

#### 1. **No Administrator Privileges Required**

Prism Visuals runs entirely in user space without requiring elevated permissions. Configuration is stored in `%APPDATA%\Prism Visuals\` and wallpapers are stored in `%USERPROFILE%\Pictures\Prism Visuals\`. The application makes no system-wide modifications except for an optional PATH environment variable addition during installation.

#### 2. **Network Security**

All HTTPS connections use TLS encryption for secure communication. User-Agent headers transparently identify the application to remote servers. A 30-second connection timeout prevents hanging connections. The application connects directly to `https://www.bing.com` for the Bing API and `https://api.unsplash.com` for the Unsplash API.

#### 3. **API Key Security**

**IMPORTANT**: Unsplash API keys are stored in **plain text** in the configuration file located at `%APPDATA%\Prism Visuals\config.json`. 

We strongly recommend the following practices:
- Keep your Unsplash API key secure and never share it
- Use the free tier API key (50 requests/hour) for personal use
- Regularly rotate your API keys
- Never commit your config.json file to version control
- Be aware that any application with access to your user profile can read this file

#### 4. **File System Operations**

The application only reads and writes to designated directories. It never modifies system files or deletes user files. Wallpapers are only added, never removed automatically. File hash verification for Bing wallpapers prevents duplicate downloads.

#### 5. **Windows API Integration**

Prism Visuals uses official Windows COM interfaces for wallpaper setting (`IDesktopWallpaper`), file picker dialog (`IFileOpenDialog`), and message boxes (`MessageBoxW`). No registry modifications occur except for installer-created entries. ANSI terminal support is enabled for colored output in the command line.

### Installer Security (MSI)

#### 1. **Installation Process**

The standard Windows Installer (MSI) package installs to `%ProgramFiles%\Prism Visuals\` and optionally adds the installation directory to the System PATH for command-line access. The installer creates Start Menu shortcuts for easy access. Registry entries are limited to installation tracking at `HKCU\Software\SibtainOcn\Prism Visuals` and PATH environment variable modification if selected during installation.

#### 2. **Uninstallation**

The uninstaller provides clean removal of all installed files, PATH entries, and Start Menu shortcuts. **Important**: User data is preserved during uninstallation. Downloaded wallpapers and configuration files are NOT deleted, allowing reinstallation without losing your data.

#### 3. **Upgrade Process**

The installer automatically detects existing installations and prevents downgrades to older versions. All user configuration and wallpapers are preserved during upgrades.

### Data Privacy

#### 1. **No Telemetry or Analytics**

Prism Visuals does not collect any usage data. There is no usage tracking, crash reporting, data collection, or transmission to third parties. The application does not monitor user behavior in any way.

#### 2. **Local-Only Configuration**

All settings are stored locally on your machine with no cloud synchronization. No account creation is required, and no personal information is collected.

#### 3. **API Usage**

**Bing**: Anonymous requests with no authentication required.

**Unsplash**: Requires a user-provided API key. Requests include your Client ID (API key), search queries (themes you specify), and image preferences (count, sort order, orientation).

#### 4. **Downloaded Content**

Wallpapers are sourced from public domain or licensed content. Bing provides Microsoft's curated daily images, while Unsplash offers photographer-submitted, royalty-free images. No private or sensitive data is downloaded.

### Third-Party Dependencies

Prism Visuals uses the following Rust crates:

| Dependency | Purpose | Security Notes |
|------------|---------|----------------|
| `reqwest` | HTTP client | Industry-standard, actively maintained |
| `serde` / `serde_json` | Configuration management | Widely used, well-audited |
| `chrono` | Date/time handling | Mature library, standard choice |
| `colored` | Terminal colors | UI only, no network access |
| `urlencoding` | URL encoding | Prevents injection attacks |
| `windows` | Windows API bindings | Official Microsoft crate |

All dependencies are sourced from [crates.io](https://crates.io) and are regularly updated.

---

## Best Practices for Users

### Secure Usage

1. **Download from Official Sources**
   - Only download Prism Visuals from the official GitHub repository
   - Verify the installer signature if available
   - Be cautious of unofficial distributions

2. **API Key Management**
   - Never share your Unsplash API key publicly
   - Don't commit configuration files to version control
   - Use API keys with appropriate rate limits
   - Rotate keys if you suspect compromise

3. **System Security**
   - Keep your Windows installation up to date
   - Use a standard user account when possible
   - Maintain antivirus/antimalware protection
   - Be cautious with PowerShell execution policies

4. **Network Security**
   - Use on trusted networks when downloading wallpapers
   - Ensure your firewall allows HTTPS connections
   - Review downloaded content before setting as wallpaper

### Configuration File Protection

The configuration file contains sensitive information:

```
%APPDATA%\Prism Visuals\config.json
```

**Contains**:
- Unsplash API key (plain text)
- Download history
- User preferences

**To protect**:
- Set restrictive file permissions if needed
- Exclude from backups to cloud services
- Delete before sharing your device

---

## Security Features

### Built-in Protections

Prism Visuals includes multiple security features to protect users and their systems. All user inputs are validated and sanitized to prevent injection attacks. Network requests have 30-second timeouts to prevent hanging connections. Unsplash API usage is tracked and rate-limited according to the service's guidelines. Hash verification prevents duplicate downloads by checking content hashes. The application handles errors gracefully without exposing sensitive information. File operations are restricted to designated directories to prevent path traversal attacks, and conservative default settings minimize security risks.

### Known Limitations

There are some security considerations users should be aware of. Unsplash API keys are stored in plain text in the configuration file, which means any application with access to your user profile can potentially read them. The security model is designed specifically for Windows, and other platforms are not supported. Currently, binaries are not code-signed, though this is planned for future releases.

---

## Compliance & Licensing

### License Compliance

Prism Visuals is proprietary software. Source code transparency is provided for security audits only. Modification, redistribution, or commercial use without permission is not allowed. See [LICENSE](LICENSE.md) for full terms.

### Content Licensing

**Bing Wallpapers** are subject to Microsoft's terms of use. **Unsplash Photos** are licensed under the [Unsplash License](https://unsplash.com/license). Users are responsible for complying with content licenses.

---

## Security Updates

### Update Policy

Security patches are released as soon as possible after verification. Critical vulnerabilities are addressed within 7 days, while non-critical issues are addressed in regular releases. Users are notified of updates via GitHub Releases.

### How to Stay Updated

To receive the latest security updates, watch this repository for new releases and check the [Releases](../../releases) page regularly. You can subscribe to GitHub notifications for automatic alerts. Always follow the update instructions provided in release notes.

---

## Contact & Support

### For Security Issues

Report vulnerabilities via [GitHub Issues](../../issues). Use the `security` label for priority handling and follow the responsible disclosure guidelines outlined above.

### For General Support

Bug reports and feature requests can be submitted via [GitHub Issues](../../issues).

---

## Acknowledgments

We appreciate the security research community and all contributors who help keep Prism Visuals secure. Thank you for using and supporting this project responsibly.

---

**Last Updated**: January 2, 2026  
**Policy Version**: 1.0

