# 🚀 Modern Open-Source SSH Client - SypherTerm

A beautiful, ultra-lightweight, and cross-platform SSH client built for developers who care about data sovereignty. Get the premium, modern UX of commercial tools like Termius, while keeping 100% control over your credentials and data.

## 👁️ The Vision

Commercial SSH clients offer beautiful interfaces and seamless cloud synchronization, but they force you to trust their closed-source infrastructure with your most sensitive data—your production server credentials and private keys. 

This project bridges that gap. It delivers a **hardware-accelerated, modern UI** while treating your data as your own. **No proprietary cloud accounts required.** You bring your own storage (Amazon S3, Google Drive, Supabase, etc.), and the client handles the rest locally and securely.

## 🎯 Core Principles

### 1. Data Sovereignty & BYOi (Bring Your Own Infrastructure)
We do not run sync servers, and we don't want your data. The application features native integrations to sync your encrypted configuration profiles directly to **your own cloud infrastructure**:
*   AWS S3 & S3-Compatible API (Cloudflare R2, MinIO, Backblaze B2)
*   Google Drive / OneDrive
*   Supabase / PostgreSQL
*   Local network shares / Git repositories

### 2. Zero-Knowledge, Local-First Security
Security is not an afterthought. 
*   **End-to-End Encryption:** All connection details, private keys, and snippets are encrypted locally using **AES-256-GCM** via a user-defined Master Password before leaving your machine.
*   **Decentralized:** Your cloud provider only sees an opaque, encrypted blob. Even if your cloud bucket is breached, your credentials remain safe.

### 3. Lightweight & High Performance
No more bloated 100MB+ Electron setups eating your RAM. 
*   Built on top of a highly optimized stack (**Tauri + Rust**) ensuring native performance, minimal memory footprint (~20-40MB RAM), and tiny executable sizes.
*   GPU-accelerated terminal rendering for lag-free scrolling and multiplexing.

### 4. Gorgeous, Developer-Centric UX
*   Modern, sleek interface featuring tab management, pane splitting (UX-driven layout), customizable themes, and vibrant typography.
*   Built-in SFTP file manager, port forwarding (tunnels) coordinator, and snippet organizer.

## 🤝 Contributing

This project is fully open-source and loves community feedback! Since we are currently in the architectural design phase, feel free to open an Issue or Discussion to talk about requested sync providers, terminal features, or UI layouts.

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
