# Pear Desktop Hotkeys

A lightweight, standalone Rust application that provides global hotkeys (Like and Dislike for now) for [Pear Desktop](https://github.com/pear-devs/pear-desktop) with visual and audio feedback.

Currently Alt+L = Like & Alt+D = Dislike

This can't be changed right now if you are using the prebuilt binary but will be possible in the next version.

If you were to build from source, you would be able to quite easily go in and change the binds.

## Features

- 🎵 **Global hotkeys** - Like/unlike songs from anywhere
- 🔔 **Smart notifications** - Shows "Liked ❤️" or "Unliked 💔" based on current state
- 🔊 **Audio feedback** - Plays a sound on each action
- ⚡ **Blazing fast** - Native Rust binary with <1ms response time
- 🪟 **Wayland compatible** - Works on modern Linux desktops
- 🔋 **Lightweight** - ~5MB RAM usage, no background services

## Requirements

- [Pear Desktop](https://github.com/pear-devs/pear-desktop) with API Server plugin enabled
- Linux (Wayland or X11)

## Installation

### Option 1: Download Pre-built Binary (Recommended for casual users)

1. Download the latest release from the [Releases page](https://github.com/CalG24/pear-hotkeys/releases/latest)
2. Extract and install:

```bash
chmod +x pear-hotkeys
sudo mv pear-hotkeys /usr/local/bin/
