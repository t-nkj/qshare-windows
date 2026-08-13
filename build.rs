fn main() {
    println!("cargo:rerun-if-changed=assets/logo.png");

    #[cfg(windows)]
    {
        let source = image::ImageReader::open("assets/logo.png")
            .expect("open application icon")
            .decode()
            .expect("decode application icon")
            .into_rgba8();
        let mut directory = ico::IconDir::new(ico::ResourceType::Icon);
        for size in [16, 24, 32, 48, 64, 128, 256] {
            let resized =
                image::imageops::resize(&source, size, size, image::imageops::FilterType::Lanczos3);
            let icon = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
            directory.add_entry(ico::IconDirEntry::encode(&icon).expect("encode application icon"));
        }
        let icon_path =
            std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("qshare.ico");
        let file = std::fs::File::create(&icon_path).expect("create ico");
        directory.write(file).expect("write ico");

        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(icon_path.to_str().expect("icon path is UTF-8"));
        resource.set("ProductName", "QShare");
        resource.set("FileDescription", "QShare for Windows");
        resource.compile().expect("compile Windows resources");
    }
}
