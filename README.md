# 📂 Rust File Manager (Actix-Web + Tailwind)

A simple, secure **web-based file manager** built with Rust and Actix-Web.  
Supports login authentication, file upload, download, preview, and delete operations — all inside a modern UI styled with **TailwindCSS**.

---

## ✨ Features
- 🔑 **Admin Login** with session-based authentication
- 📂 **Directory Listing** (navigable with back button)
- ⬆️ **File Uploads** to any folder
- ⬇️ **File Downloads**
- 👀 **File Preview** (images, videos, audio, text, HTML)
- 🗑 **File & Folder Delete**
- ⏳ **Failed login attempt lockout** (3 tries → 30 min block) *(optional)*
- 🎨 **Responsive UI** using **TailwindCSS + FontAwesome**

---

## 🛠 Tech Stack
- [Rust](https://www.rust-lang.org/) (Backend)
- [Actix-Web](https://actix.rs/) (Web framework)
- [Actix-Session](https://docs.rs/actix-session/latest/actix_session/) (Authentication)
- [Actix-Files](https://docs.rs/actix-files/latest/actix_files/) (Static file serving)
- [TailwindCSS](https://tailwindcss.com/) (UI)
- [FontAwesome](https://fontawesome.com/) (Icons)

---
## Configuretion 
**Default upload directory**
static UPLOAD_DIR: &str = "C:\\Users\\ANKIT\Desktop\\"; //Update path

**Admin credentials**
static USERNAME: &str = "admin"; // Change username 
static PASSWORD: &str = "1234"; //Change Password


## 🚀 Running the Project

### 1. Clone Repository
```bash
git clone https://github.com/ankit78v/rust-file-manager.git
cd rust-file-manager
//Install Dependencies
rustup update
// Run Server
cargo run
