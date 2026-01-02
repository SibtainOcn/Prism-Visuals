# Prism Visuals CLI

**A professional CLI tool for downloading and managing high-quality wallpapers automatically**

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![License](https://img.shields.io/badge/license-Proprietary-red.svg)

![Platform](https://img.shields.io/badge/platform-Windows-blue.svg)



## Installation Note

When running the installer, Windows SmartScreen may show a warning:
"Windows protected your PC - Unknown publisher"

**This is normal for new software.** To install:

1. Click **"More info"**
2. Click **"Run anyway"**

The software is safe and open-source. You can review the code before installing.

**Why this happens:** The installer isn't code-signed yet (costs $300+/year). 
As more people download it, Windows will build trust automatically.




## Quick Start


## Commands

When in interactive mode, you'll see a prompt like this where you can enter commands directly without the `visuals` prefix:


| Command | Shortcut | Description |
|---------|----------|-------------|
| `visuals fetch` | `f` | Download wallpapers from current source |
| `visuals change` | `c` | Choose & set wallpaper |
| `visuals open` | `o` | Open wallpaper folder |
| `visuals source` | `src` | Switch source (Bing/Unsplash) |
| `visuals reset` | `r` | Reset all settings to default |
| `visuals help` | `h, q`, | Show this help |
| `visuals exit` | - | Exit program |
| `visuals` | - | Interactive mode |








### Simply open powershell and RUN `visuals` 


### Interactive Mode
```bash
# Run without arguments


`
+------------------------------------------+
| >    YOUR COMMAND HERE                   |
+------------------------------------------+
`

# Use shortcuts
> f     # fetch
> c     # change
> o     # open folder
> src   # source setup
> q     # exit
> r     # RESET EVERYTHING to default
> h      # help

```


---

## Fetch Workflow

### Switch Source
```bash
visuals source
```
Toggle between Bing (daily curated) and Unsplash (themed) wallpaper sources.


### Bing Source
1. Checks last 8 days for new images
2. Downloads only new/unique images (hash-based)
3. Automatically goes to archive (-7, -14, -21 days) if no new images
4. Resets to latest when new images appear

### Unsplash Source
1. Prompts for theme (or random)
2. Prompts for image count (5-30)
3. Prompts for sort method:
   - **Relevance**: Most popular/best quality (default)
   - **Latest**: Newest photos first
   - **Random**: Truly random selection
4. Downloads selected images
5. Tracks rate limit usage

---

## Sort Options Explained

### Relevance (Recommended)
- Returns most popular and highest quality images
- Best match for your search theme
- Consistently beautiful results


### Latest
- Returns newest photos first
- Fresh content from recent uploads
- Modern aesthetics
- Good for trending themes

### Random
- Truly random selection from all time periods
- Variety across different eras
- Good for discovering hidden gems


---

## Troubleshooting

### "Invalid Unsplash API key"
- Verify your API key is correct
- Get a new key at [Unsplash Developers](https://unsplash.com/developers)
- Set it using: `fetch YOUR_KEY`

### "Rate limit exceeded"
- MODE allows 50 requests/hour
- Wait 1 hour before trying again
- for paid api key 5,000 requests/hour  [Unsplash Developers](https://unsplash.com/developers)

### "No photos found for this theme"
- Try a different theme
- Use "random" for general wallpapers
- Check your internet connection

### "Cannot find Pictures directory"
- Ensure `%USERPROFILE%\Pictures` exists
- Create it manually if needed

---

## FAQ

**Q: Does it automatically change wallpapers?**  
A: No, you manually select wallpapers using the `change` command for full control.

**Q: How many wallpapers can I download?**  
A: Unlimited! The tool only adds wallpapers, never deletes them. API LIMITS APPLY.

**Q: What's the difference between Bing and Unsplash?**  
A: Bing provides daily curated images (no setup). Unsplash offers themed photos with more control (requires API key).

**Q: Can I use both sources?**  
A: Yes! Switch between them using `visuals source`.

**Q: Where are wallpapers stored?**  
A: `%USERPROFILE%\Pictures\Prism Visuals\`

**Q: Do I need admin rights?**  
A: No, the tool works without administrator privileges.

---

## Technical Details

### Dependencies
- `reqwest` - HTTP client
- `serde` / `serde_json` - Configuration management
- `chrono` - Date/time handling
- `colored` - Terminal colors
- `urlencoding` - URL encoding
- `windows` - Windows API integration


---

## License

This project is under a Proprietary License. See the [LICENSE](LICENSE) file for details.

---

- **Built with**: Rust
- **dev ~ SibtainOcn**


---

**Enjoy your beautiful wallpapers!**
