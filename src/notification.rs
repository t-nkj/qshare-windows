#[derive(Clone, Default)]
pub struct Notifier;

impl Notifier {
    pub fn new() -> Self {
        Self
    }

    pub fn progress(&self, title: &str, completed: u64, total: u64) {
        let percent = percentage(completed, total);
        platform::show(title, "全体の進捗", Some(percent));
    }

    pub fn success(&self, message: &str) {
        platform::show("QShare", message, None);
    }

    pub fn error(&self, message: &str) {
        platform::show_error(message);
    }

    pub fn setup_required(&self, env_path: &std::path::Path) {
        platform::show_setup(env_path);
    }
}

pub fn percentage(completed: u64, total: u64) -> u8 {
    if total == 0 {
        return 100;
    }
    ((completed.saturating_mul(100) / total).min(100)) as u8
}

#[cfg(windows)]
mod platform {
    use std::sync::OnceLock;

    use windows::{
        Data::Xml::Dom::XmlDocument,
        UI::Notifications::{
            NotificationData, NotificationUpdateResult, ToastNotification, ToastNotificationManager,
        },
        Win32::{
            Storage::EnhancedStorage::PKEY_AppUserModel_ID,
            System::{
                Com::{
                    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
                    IPersistFile,
                    StructuredStorage::{
                        PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
                    },
                },
                Variant::VT_LPWSTR,
            },
            UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore},
        },
        core::{GUID, HSTRING, Interface, PWSTR},
    };

    const APP_ID: &str = "QShare";
    static SHORTCUT_REGISTERED: OnceLock<()> = OnceLock::new();

    fn notifier() -> windows::core::Result<windows::UI::Notifications::ToastNotifier> {
        if SHORTCUT_REGISTERED.get().is_none() {
            ensure_shortcut()?;
            let _ = SHORTCUT_REGISTERED.set(());
        }
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))
    }

    fn ensure_shortcut() -> windows::core::Result<()> {
        crate::logging::debug("Toast shortcut registration started");
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
        crate::logging::debug("Toast shortcut COM apartment initialized");
        let app_data = std::env::var_os("APPDATA").ok_or_else(|| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80070002_u32 as i32),
                "APPDATA is unavailable",
            )
        })?;
        let shortcut_path = std::path::PathBuf::from(app_data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("QShare.lnk");
        if shortcut_path.exists() {
            crate::logging::debug("Toast Start menu shortcut already registered: AUMID=QShare");
            return Ok(());
        }
        let executable = std::env::current_exe().map_err(|error| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80004005_u32 as i32),
                format!("Cannot resolve QShare executable: {error}"),
            )
        })?;
        let shell_link: IShellLinkW = unsafe {
            CoCreateInstance(
                &GUID::from_u128(0x00021401_0000_0000_c000_000000000046),
                None,
                CLSCTX_INPROC_SERVER,
            )
        }?;
        crate::logging::debug("Toast shortcut ShellLink created");
        let executable = HSTRING::from(executable.to_string_lossy().as_ref());
        unsafe { shell_link.SetPath(&executable)? };
        crate::logging::debug("Toast shortcut target set");
        let property_store: IPropertyStore = shell_link.cast()?;
        crate::logging::debug("Toast shortcut property store acquired");
        let app_id = HSTRING::from(APP_ID);
        let property_value = string_propvariant(&app_id);
        unsafe {
            property_store.SetValue(&PKEY_AppUserModel_ID, &property_value)?;
            crate::logging::debug("Toast shortcut AUMID property set");
            property_store.Commit()?;
        }
        // The PROPVARIANT borrows the HSTRING buffer; it must not clear that borrowed pointer.
        std::mem::forget(property_value);
        crate::logging::debug("Toast shortcut properties committed");
        let persist_file: IPersistFile = shell_link.cast()?;
        crate::logging::debug("Toast shortcut persistence interface acquired");
        let shortcut = HSTRING::from(shortcut_path.to_string_lossy().as_ref());
        unsafe { persist_file.Save(&shortcut, true)? };
        crate::logging::debug("Toast Start menu shortcut registered: AUMID=QShare");
        Ok(())
    }

    fn string_propvariant(value: &HSTRING) -> PROPVARIANT {
        PROPVARIANT {
            Anonymous: PROPVARIANT_0 {
                Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                    vt: VT_LPWSTR,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: PROPVARIANT_0_0_0 {
                        pwszVal: PWSTR(value.as_ptr() as *mut _),
                    },
                }),
            },
        }
    }

    pub fn show(title: &str, message: &str, progress: Option<u8>) {
        if let Err(error) = show_inner(title, message, progress, "transfer") {
            log_toast_error("show", &error);
        }
    }

    pub fn show_setup(env_path: &std::path::Path) {
        if let Err(error) = show_setup_inner(env_path) {
            log_toast_error("show_setup", &error);
        }
    }

    pub fn show_error(message: &str) {
        if let Err(error) = show_inner("QShare エラー", message, None, "error") {
            log_toast_error("show_error", &error);
        }
    }

    fn show_inner(
        title: &str,
        message: &str,
        progress: Option<u8>,
        toast_tag: &str,
    ) -> windows::core::Result<()> {
        if let Some(value) = progress {
            match update_progress(value) {
                Ok(NotificationUpdateResult::Succeeded) => {
                    crate::logging::debug(&format!("Toast progress update succeeded: {value}%"));
                    return Ok(());
                }
                Ok(result) => crate::logging::debug(&format!(
                    "Toast progress update result: {result:?}; creating a new toast"
                )),
                Err(error) => log_toast_error("update_progress", &error),
            }
        }
        let progress_xml = if progress.is_some() {
            "<progress value=\"{progressValue}\" valueStringOverride=\"{progressText}\"/>"
        } else {
            ""
        };
        let xml = format!(
            "<toast launch=\"qshare\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text>{}</binding></visual></toast>",
            escape(title),
            escape(message),
            progress_xml
        );
        crate::logging::debug(&format!(
            "Toast XML prepared: kind={toast_tag}, progress={}",
            progress.is_some()
        ));
        let document = XmlDocument::new()?;
        crate::logging::debug("Toast XmlDocument created");
        document.LoadXml(&HSTRING::from(xml))?;
        crate::logging::debug("Toast XML loaded");
        let toast = ToastNotification::CreateToastNotification(&document)?;
        crate::logging::debug("Toast notification created");
        toast.SetTag(&HSTRING::from(toast_tag))?;
        toast.SetGroup(&group())?;
        crate::logging::debug("Toast tag and group set");
        if let Some(value) = progress {
            toast.SetData(&progress_data(value)?)?;
            crate::logging::debug("Toast progress data set");
        }
        let notifier = notifier()?;
        crate::logging::debug("Toast notifier created: AUMID=QShare");
        notifier.Show(&toast)?;
        crate::logging::info(&format!("Toast show succeeded: kind={toast_tag}"));
        Ok(())
    }

    fn show_setup_inner(env_path: &std::path::Path) -> windows::core::Result<()> {
        let Ok(file_url) = url::Url::from_file_path(env_path) else {
            return Ok(());
        };
        let file_url = escape(file_url.as_str());
        let xml = format!(
            "<toast activationType=\"protocol\" launch=\"{file_url}\"><visual><binding template=\"ToastGeneric\"><text>QShareの初期設定が必要です</text><text>.env を編集して QSHARE_TOKEN を設定してください</text></binding></visual><actions><action content=\"編集する\" activationType=\"protocol\" arguments=\"{file_url}\"/></actions></toast>"
        );
        crate::logging::debug("Setup toast XML prepared: kind=setup, activation=protocol");
        let document = XmlDocument::new()?;
        crate::logging::debug("Setup toast XmlDocument created");
        document.LoadXml(&HSTRING::from(xml))?;
        crate::logging::debug("Setup toast XML loaded");
        let toast = ToastNotification::CreateToastNotification(&document)?;
        crate::logging::debug("Setup toast notification created");
        toast.SetTag(&tag())?;
        toast.SetGroup(&group())?;
        crate::logging::debug("Setup toast tag and group set");
        let notifier = notifier()?;
        crate::logging::debug("Setup toast notifier created: AUMID=QShare");
        notifier.Show(&toast)?;
        crate::logging::info("Toast show succeeded: kind=setup");
        Ok(())
    }

    fn update_progress(value: u8) -> windows::core::Result<NotificationUpdateResult> {
        notifier()?.UpdateWithTagAndGroup(&progress_data(value)?, &tag(), &group())
    }

    fn log_toast_error(operation: &str, error: &windows::core::Error) {
        crate::logging::error(&format!(
            "Toast {operation} failed: HRESULT=0x{:08X}, message={error}",
            error.code().0 as u32
        ));
    }

    fn progress_data(value: u8) -> windows::core::Result<NotificationData> {
        let data = NotificationData::new()?;
        let values = data.Values()?;
        let _ = values.Insert(
            &HSTRING::from("progressValue"),
            &HSTRING::from(format!("{}", value as f32 / 100.0)),
        )?;
        let _ = values.Insert(
            &HSTRING::from("progressText"),
            &HSTRING::from(format!("全体 {value}%")),
        )?;
        Ok(data)
    }

    fn tag() -> HSTRING {
        HSTRING::from("transfer")
    }

    fn group() -> HSTRING {
        HSTRING::from(format!("qshare-{}", std::process::id()))
    }

    fn escape(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\"', "&quot;")
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn show(_title: &str, _message: &str, _progress: Option<u8>) {}

    pub fn show_error(_message: &str) {}

    pub fn show_setup(_env_path: &std::path::Path) {}
}

#[cfg(test)]
mod tests {
    use super::percentage;

    #[test]
    fn calculates_progress_for_empty_transfer() {
        assert_eq!(percentage(0, 0), 100);
    }

    #[test]
    fn caps_progress_at_one_hundred() {
        assert_eq!(percentage(150, 100), 100);
    }
}
