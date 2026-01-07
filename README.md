<div align="center">

#  Prism Visuals

**Transform your desktop with stunning wallpapers from the world's best sources**

[![Version](https://img.shields.io/badge/version-1.2.6-blue.svg?style=for-the-badge)](https://github.com/SibtainOcn/Prism-Visuals/releases)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/windows-10%2F11-0078D6.svg?style=for-the-badge&logo=windows)](https://www.microsoft.com/windows)
[![License](https://img.shields.io/badge/license-Proprietary-red.svg?style=for-the-badge)](LICENSE)

[![Downloads](https://img.shields.io/github/downloads/SibtainOcn/Prism-Visuals/total.svg?style=for-the-badge&color=success&cacheSeconds=60)](https://github.com/SibtainOcn/Prism-Visuals/releases)
[![Stars](https://img.shields.io/github/stars/SibtainOcn/Prism-Visuals?style=for-the-badge&color=yellow&cacheSeconds=60)](https://github.com/SibtainOcn/Prism-Visuals)
[![Release](https://img.shields.io/github/v/release/SibtainOcn/Prism-Visuals?style=for-the-badge&color=purple&cacheSeconds=60)](https://github.com/SibtainOcn/Prism-Visuals/releases/latest)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-☕-FFDD00?style=for-the-badge)](https://buymeacoffee.com/SibtainOcn)

---

**4 Sources** · **Auto-Change** · **Endless Exploration** · **Zero Setup**

[Download](#installation) · [Documentation](#quick-start) · [Issues](https://github.com/SibtainOcn/Prism-Visuals/issues)

</div>

---

## Quick Start

```powershell
# Open PowerShell and run
visuals
```

### quick commands
```
> f      # Fetch new wallpapers
> c      # Change current wallpaper
> s      # Setup auto-change schedule
> src    # Switch wallpaper source
> h      # Full help
> exit   # Exit program
```

---

## Sources

| Source | API Key | Rate Limit | Best For |
|--------|:-------:|:----------:|----------|
| **Spotlight** | `Free` | Unlimited | Windows 4K curated |
| **Spotlight Archive** | `Free` | Manual | Browse & pick from 10,000+ |
| **Wallhaven** | `Free` | 45/min | HD wallpaper variety |
| **Unsplash** | `Required` | 50/hr | Themed photography |
| **Pexels** | `Required` | 200/hr | Professional quality |

> **Tip:** Start with Spotlight or Wallhaven - no setup required!

### When to Use What

| Use Case | Best Source |
|----------|-------------|
| Just want beautiful wallpapers automatically | **Spotlight** (`f`) |
| Want to pick specific images you like | **Universal Pick** (`p`) - 4 sources! |
| Want variety with tags/themes, no API key | **Wallhaven** |
| Want themed photography (nature, city) | **Unsplash** |
| Want professional studio-quality photos | **Pexels** |

> **Pick Mode** (`p`) opens browser + CLI for manual browsing from any of 4 sources - no API keys needed!

---

## Commands

| Category | Command | Shortcut | Description |
|----------|---------|:--------:|-------------|
| **Core** | `fetch` | `f` | Download wallpapers |
| | `change` | `c` | Choose & set wallpaper |
| | `open` | `o` | Open wallpaper folder |
| | `source` | `src` | Switch source |
| **Schedule** | `set` | `s` | Enable auto-change |
| | `unset` | `un` | Disable auto-change |
| | `status` | `ss` | Check schedule |
| **Archive** | `pick` | `p` | Download from Spotlight Archive |
| **System** | `help` | `h` | Show help |
| | `exit` | `quit` | Exit program |
| | `0` | - | Go back (in menus) |

---

## Auto-Change Schedule

| Frequency | Description |
|-----------|-------------|
| Daily | Fresh wallpaper every morning |
| Hourly | New look every hour |
| Custom | 1-24 hour intervals |

```powershell
visuals set    # Enable auto-change
visuals status # Check schedule
visuals unset  # Disable
```

> **[INFO]** Auto-change uses your currently selected source. Change source anytime with `src` command!

---

## Troubleshooting

<details>
<summary><b>"Invalid API key"</b></summary>

1. Use `rm` command to reset your API key
2. Get a new key from [Unsplash](https://unsplash.com/developers) or [Pexels](https://www.pexels.com/api/new/)
3. Run `src` to set the new key
</details>

<details>
<summary><b>"Rate limit exceeded"</b></summary>

| Source | Wait Time |
|--------|-----------|
| Unsplash | 1 hour |
| Pexels | 1 hour |
| Wallhaven | 1 minute |
</details>

---

## Storage

Wallpapers are saved to: `%USERPROFILE%\Pictures\Prism Visuals\`

## Support

> Your support helps keep the project alive and growing!


<div align="center">





<a href="https://buymeacoffee.com/SibtainOcn"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50"></a>


</div>


---

<div align="center">

**Built with ❤️ in Rust**

Made by [SibtainOcn](https://github.com/SibtainOcn)

[![GitHub](https://img.shields.io/badge/GitHub-Follow-181717?style=flat-square&logo=github)](https://github.com/SibtainOcn)

</div>
