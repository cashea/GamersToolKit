fn main() {
    // Only run on Windows
    #[cfg(target_os = "windows")]
    {
        // Embed the application icon into the executable
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "GamersToolKit");
        res.set("FileDescription", "Real-time game analysis and assistance overlay");
        res.set("LegalCopyright", "Copyright (c) 2024");
        res.compile().expect("Failed to compile Windows resources");
    }
}
