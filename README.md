# Visuals CLI

**A fast, minimal CLI built to refresh your desktop effortlessly**

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![License](https://img.shields.io/badge/license-Proprietary-red.svg)

![Platform](https://img.shields.io/badge/platform-Windows-blue.svg)



## Features




## Quick Start

> OPEN POWERSHELL and run `visuals` to see the **Main Menu** 


### Interactive Mode
```bash
# Run without arguments - shows Main Menu by default
visuals

+------------------------------------------+
| >    YOUR COMMAND HERE                   |
+------------------------------------------+


> f     # fetch
> c     # change
> o     # open folder
> src   # source setup
> s     # schedule auto-change
> h     # full help
> v     # main menu
> q     # exit

```


---

## Fetch Workflow

### Switch Source
```bash
visuals source
```
Choose from 4 wallpaper sources:

| Source | API Key | Best For |
|--------|---------|----------|
| **Bing** | Not needed | Daily hand-picked visuals |
| **Wallhaven** | Not needed | Exploring diverse styles |
| **Unsplash** | Required | Curated aesthetics & moods |
| **Pexels** | Required | High quality premium visuals |

----

> *You can search your desire imgs from Wallhaven, Unsplash, Pexels*

> *First run `src` then run `fetch` to download imgs*

> **Note:** *API keys are free to get from their respective websites* -
> *limits apply.*


### Full Command Reference

| Category | Command | Shortcut | Description |
|----------|---------|----------|-------------|
| **Core** | `fetch` | `f` | Download wallpapers |
| | `change` | `c` | Choose & set wallpaper |
| | `open` | `o` | Open wallpaper folder |
| | `source` | `src` | Switch source (4 options) |
| | `reset` | `r` | Reset all settings |
| | `rm` | - | Reset current source API key only |
| **Schedule** | `set` | `s` | Enable auto-change schedule |
| | `unset` | `un` | Disable auto-change |
| | `status` | `ss` | Check schedule status |
| **System** | `help` | `h`, `?` | Show full help |
| | `menu` | `v`, `m`, `visuals` | Show main menu |
| | `update` | - | Check & install updates |
| | `exit` | `q` | Exit program |


> ## For autochange  

Automatically refresh your desktop wallpapers on a schedule.

| Command | Shortcut | Description |
|---------|----------|-------------|
| `visuals set` | `s` | Enable auto-change schedule |
| `visuals unset` | `un` | Disable auto-change |
| `visuals status` | `st` | Check schedule status |



### Scheduling Options
| Option | Frequency |
|--------|----------|
| Daily | At specific time (e.g., 9:00 AM) |
| Every 6 hours | 4 times per day |
| Every 3 hours | 8 times per day |
| Hourly | Every hour |
| Custom | 1-24 hours interval |

## Troubleshooting

### "Invalid API key" (Unsplash/Pexels)
- Verify your API key is correct
- Use **`rm`** command to clear and reset your API key
- Get a new key:
  - Unsplash: [unsplash.com/developers](https://unsplash.com/developers)
  - Pexels: [pexels.com/api](https://www.pexels.com/api/new/)
- Then run `src` to set the new key

### "Rate limit exceeded"
- Unsplash: 50 requests/hour (wait 1 hour)
- Pexels: 200 requests/hour (wait 1 hour)
- Wallhaven: 45 requests/minute (wait 1 minute)
- Bing: 8 images/day (wait until next day)

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



---


---

## License

This project is under a Proprietary License. See the [LICENSE](LICENSE) file for details.

---

- **Built with**: Rust
- **dev ~ SibtainOcn**


---

**Enjoy your beautiful wallpapers!**
