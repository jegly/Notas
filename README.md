<p align="center">
  <img src="https://github.com/globalcve/NocturneNotes/blob/main/NocturneNotesBanner.png" alt="Nocturne Notes Banner" width="600"/>
</p>

<p align="center">
  <!-- Language badges -->
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust Badge"/>
  <img src="https://img.shields.io/badge/GTK4-7D7D7D?style=for-the-badge&logo=gtk&logoColor=white" alt="GTK4 Badge"/>
  <img src="https://img.shields.io/badge/Linux-333333?style=for-the-badge&logo=linux&logoColor=white" alt="Linux Badge"/>
</p>

---

## 📝 Nocturne Notes

**Nocturne Notes** is a secure, modern note‑taking application built with Rust and GTK4.  
It encrypts all notes using **AES‑256‑GCM** with **Argon2** password‑based key derivation, ensuring your data stays private.  
The app features a clean GTK4 interface, reproducible Debian packaging, and a focus on maintainable, open‑source development.

---

## 🚀 Build Instructions

Clone the repo and run:

```bash
chmod +x build.sh
./build.sh


This will produce nocturne-notes-deb.deb in the repo root.
Install with:

sudo apt install ./nocturne-notes-deb.deb
```


Prebuilt .deb packages are available under Releases.


📄 License - MIT

