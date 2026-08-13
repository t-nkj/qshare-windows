fn main() {
    println!("cargo:rerun-if-changed=assets/logo.png");

    #[cfg(windows)]
    {
        let source = image::ImageReader::open("assets/logo.png")
            .expect("open application icon")
            .decode()
            .expect("decode application icon")
            .into_rgba8();
        let icon =
            ico::IconImage::from_rgba_data(source.width(), source.height(), source.into_raw());
        let mut directory = ico::IconDir::new(ico::ResourceType::Icon);
        directory.add_entry(ico::IconDirEntry::encode(&icon).expect("encode application icon"));
        let icon_path =
            std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("qshare.ico");
        let file = std::fs::File::create(&icon_path).expect("create ico");
        directory.write(file).expect("write ico");

        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(icon_path.to_str().expect("icon path is UTF-8"));
        resource.compile().expect("compile Windows resources");
    }
}
