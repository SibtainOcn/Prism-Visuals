# Security Policy

## Supported Versions

| Version | Status |
|---------|--------|
| 1.2.x   | Supported |
| 1.1.x   | Supported |
| 1.0.x   | Supported |
| < 1.0   | Not Supported |

Always use the latest version for the most up-to-date security patches.

---

## Reporting a Vulnerability

We take security seriously. If you discover a vulnerability, please follow responsible disclosure:

### How to Report

1. Open a [GitHub Issue](../../issues) with the `security` label
2. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Affected versions

### Response Timeline

| Priority | Response Time |
|----------|---------------|
| Critical | 24-48 hours |
| High | 72 hours |
| Medium | 7 days |
| Low | Next release |

### Guidelines

- Do not publicly disclose before we address the issue
- Do not exploit the vulnerability
- Do not access data that doesn't belong to you

---

## Security Model

### No Elevated Privileges

Prism Visuals runs entirely in user space. No administrator rights required.

| Data Type | Location |
|-----------|----------|
| Configuration | `%APPDATA%\Prism Visuals\` |
| Wallpapers | `%USERPROFILE%\Pictures\Prism Visuals\` |

### Network Security

- All connections use HTTPS/TLS encryption
- 30-second connection timeout
- Direct API connections only (no proxies or third-party services)

| API | Endpoint |
|-----|----------|
| Spotlight (Microsoft) | `https://arc.msn.com` |
| Spotlight Archive | `https://windows10spotlight.com` |
| Unsplash | `https://api.unsplash.com` |
| Wallhaven | `https://wallhaven.cc` |
| Pexels | `https://api.pexels.com` |

### API Key Storage

API keys are stored in plain text in `config.json`.

**Best Practices:**
- Keep API keys private
- Never commit config.json to version control
- Rotate keys regularly
- Use free tier keys for personal use

### File Operations

- Read/write restricted to designated directories
- No system file modifications
- No automatic file deletion
- Hash verification prevents duplicates

---

## Privacy

### No Telemetry

Prism Visuals does not collect:
- Usage data
- Crash reports
- Analytics
- Personal information

### Local-Only Storage

- All settings stored locally
- No cloud synchronization
- No account required

### API Data

| Source | Data Sent |
|--------|-----------|
| Spotlight (Microsoft) | User-Agent header only |
| Spotlight Archive | User-Agent header only |
| Unsplash | API key, search query |
| Wallhaven | Search query, preferences |
| Pexels | API key, search query |

### Universal Pick Mode

Pick mode (`p`) scrapes public web pages - no API keys required:

| Source | Method |
|--------|--------|
| Spotlight Archive | HTML parsing |
| Unsplash | Download endpoint |
| Pexels | HTML parsing |
| Wallhaven | Direct URL construction |

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client |
| `serde` | Configuration |
| `chrono` | Date/time |
| `windows` | Windows API (official Microsoft crate) |
| `colored` | Terminal output |

All dependencies sourced from [crates.io](https://crates.io).

---

## Installer (MSI)

### Installation

- Installs to `%ProgramFiles%\Prism Visuals\`
- Optional PATH modification
- Start Menu shortcuts

### Uninstallation

- Complete removal of installed files
- User data preserved (wallpapers, config)
- Clean registry removal

---

## Best Practices

### For Users

1. Download only from official GitHub releases
2. Keep Windows updated
3. Protect your API keys
4. Use trusted networks

### Configuration Protection

Location: `%APPDATA%\Prism Visuals\config.json`

Contains:
- API keys (plain text)
- User preferences
- Download history

---

## Updates

Security patches are prioritized and released promptly.

- Watch this repository for releases
- Check [Releases](../../releases) regularly
- Follow update instructions in release notes

---

## Contact

- Security Issues: [GitHub Issues](../../issues) with `security` label
- General Support: [GitHub Issues](../../issues)

---

**Last Updated**: January 6, 2026  
**Policy Version**: 1.2
