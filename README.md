# Notas
<p align="center">
<img src="assets/Notas-Animated.gif" alt="Notas Animation" />
</p>

---

<p align="center">
<img src="assets/Welcome_Screen_Dark.png" alt="Welcome Dark" width="45%"/>
<img src="assets/Welcome_Screen_White.png" alt="Welcome White" width="45%"/>
</p>

---

<p align="center">
<img src="assets/Darkmode.png" alt="Dark Mode" width="45%"/>
<img src="assets/Lightmode.png" alt="Light Mode" width="45%"/>
</p>

<p align="center">
<img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust Badge"/>
<img src="https://img.shields.io/badge/GTK4-7D7D7D?style=for-the-badge&logo=gtk&logoColor=white" alt="GTK4 Badge"/>
<img src="https://img.shields.io/badge/Linux-333333?style=for-the-badge&logo=linux&logoColor=white" alt="Linux Badge"/>
</p>

---

A minimalist, secure note-taking application for Linux. Built with Rust and GTK4, featuring AES-256 encryption and password protection at startup.

## Install
```bash
sudo dpkg -i notas_2.1.0_amd64.deb
```

The DotGothic16 font is bundled and installed automatically.

## Build from source

Requirements:
```bash
sudo apt install pkg-config libgtk-4-dev libadwaita-1-dev
```
```bash
./build-deb.sh
```

## Security

- **AES-256-GCM** encryption for all notes
- **Argon2id** key derivation (memory-hard, resistant to GPU attacks)
- **Auto-lock** after configurable timeout
- **Clipboard auto-clear** after copying sensitive content
- All data encrypted locally — nothing leaves your device
- No ads / no tracking / no internet connectivity

## Uninstall
```bash
sudo apt remove notas
```

User data is removed automatically on uninstall so a reinstall always starts clean.

## License

Copyright © 2025 JEGLY. All rights reserved.

This software is provided for personal use only. Redistribution, modification, or commercial use of the source code is prohibited without written permission.
