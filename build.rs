fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/veylo.ico");
        res.set("ProductName", "Veylo");
        res.set("FileDescription", "Veylo - Intelligent Offline Voice Dictation");
        res.set("CompanyName", "Veylo AI");
        res.set("LegalCopyright", "Copyright (c) 2026 Abneesh Singh");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource icon: {}", e);
        }
    }
}
