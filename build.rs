fn main() {
    println!("cargo:rerun-if-changed=assets/logo.png");
    println!("cargo:rerun-if-env-changed=GITHUB_RUN_NUMBER");
    println!("cargo:rustc-env=QSHARE_BUILD_VERSION={}", build_version());

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

fn build_version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    if let Ok(run_number) = std::env::var("GITHUB_RUN_NUMBER")
        && !run_number.is_empty()
    {
        return format!("v{version}-build.{run_number}");
    }

    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let (year, month, day, hour, minute, second) = timestamp_components(seconds + 9 * 3_600);
    format!("v{version}-custom{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}")
}

fn timestamp_components(seconds: i64) -> (i32, u32, u32, i64, i64, i64) {
    let days = seconds.div_euclid(86_400);
    let remaining = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    (
        year,
        month,
        day,
        remaining / 3_600,
        (remaining % 3_600) / 60,
        remaining % 60,
    )
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}
