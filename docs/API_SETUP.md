# API Keys Setup Guide

This guide will help you get free API keys for premium wallpaper sources.

---

## Unsplash

**Official Developer Portal:** https://unsplash.com/developers

### Steps to Get API Key

1. Go to https://unsplash.com/developers
2. Click **"Register as a Developer"** (or sign in if you already have an account)
3. Click **"New Application"**
4. Accept the API terms
5. Fill in application details:
   - **Application name:** `Prism Visuals` (or any name)
   - **Description:** `Personal wallpaper manager`
6. Click **"Create Application"**
7. Copy your **Access Key** (starts with `...`)

### Rate Limits (Free Tier)
- **50 requests per hour**
- **5,000 requests per month**

---

## Pexels

**Official API Portal:** https://www.pexels.com/api/

### Steps to Get API Key

1. Go to https://www.pexels.com/api/
2. Click **"Get Started"** or **"Sign Up"**
3. Create a free account (or sign in)
4. Go to your **API dashboard**: https://www.pexels.com/api/new/
5. Fill in:
   - **Project name:** `Prism Visuals`
   - **Project description:** `Desktop wallpaper application`
6. Click **"Generate API Key"**
7. Copy your **API Key**

### Rate Limits (Free Tier)
- **200 requests per hour**
- Unlimited monthly requests

---

## Adding Keys to Prism Visuals

Once you have your API keys:

1. Run `visuals` in your terminal
2. Type `src` to open source selection
3. Choose your desired source (Unsplash/Pexels/Wallhaven)
4. Paste your API key when prompted
5. Done! Start fetching with `f`

---

## Troubleshooting

### "Invalid API key" error
- Double-check you copied the entire key (no extra spaces)
- Use `rm` command to reset and re-enter the key

### Rate limit exceeded
- Wait for the cooldown period (see rate limits above)
- Switch to a different source temporarily

### Need help?
- Open an issue: https://github.com/SibtainOcn/Prism-Visuals/issues
- Check the main help: Run `visuals help`
