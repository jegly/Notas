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
sudo apt install pkg-config libgtk-4-dev libadwaita-1-dev rustup
```
```bash
./build-deb.sh
```

## Security

- **AES-256-GCM** encryption for all notes
- **Argon2id** key derivation — memory-hard, resistant to GPU and brute-force attacks
- **Unique salt per vault** — every installation has a cryptographically random salt
- **Fresh nonce per save** — every write uses a new random nonce, no nonce reuse
- **In-memory zeroization** — decrypted note content is wiped from memory when the app locks
- **mlock protection** — sensitive key material is locked in RAM, preventing it from being swapped to disk
- **Auto-lock** after configurable timeout
- **Clipboard auto-clear** after copying sensitive content
- **Clean uninstall** — vault and config are wiped on removal so no data lingers
- All data encrypted locally — nothing leaves your device
- No ads / no tracking / no internet connectivity

## Uninstall
```bash
sudo apt remove notas
```

User data is removed automatically on uninstall so a reinstall always starts clean.

## License

MIT License

Copyright © 2025 JEGLY. All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
