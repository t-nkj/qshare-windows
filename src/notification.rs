#[derive(Clone, Default)]
pub struct Notifier;

impl Notifier {
    pub fn new() -> Self {
        Self
    }

    pub fn progress(&self, title: &str, completed: u64, total: u64) {
        let percent = percentage(completed, total);
        platform::show(title, &format!("{percent}%"), Some(percent));
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
    use windows::{
        Data::Xml::Dom::XmlDocument,
        UI::Notifications::{
            NotificationData, NotificationUpdateResult, ToastNotification, ToastNotificationManager,
        },
        core::HSTRING,
    };

    const APP_ID: &str = "QShare";

    pub fn show(title: &str, message: &str, progress: Option<u8>) {
        if let Err(error) = show_inner(title, message, progress, "transfer") {
            eprintln!("notification error: {error}");
        }
    }

    pub fn show_setup(env_path: &std::path::Path) {
        if let Err(error) = show_setup_inner(env_path) {
            eprintln!("notification error: {error}");
        }
    }

    pub fn show_error(message: &str) {
        let _ = show_inner("QShare エラー", message, None, "error");
    }

    fn show_inner(
        title: &str,
        message: &str,
        progress: Option<u8>,
        toast_tag: &str,
    ) -> windows::core::Result<()> {
        if let Some(value) = progress
            && update_progress(value).unwrap_or(NotificationUpdateResult::NotificationNotFound)
                == NotificationUpdateResult::Succeeded
        {
            return Ok(());
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
        let document = XmlDocument::new()?;
        document.LoadXml(&HSTRING::from(xml))?;
        let toast = ToastNotification::CreateToastNotification(&document)?;
        if progress.is_some() {
            toast.SetTag(&HSTRING::from(toast_tag))?;
            toast.SetGroup(&group())?;
        }
        if let Some(value) = progress {
            toast.SetData(&progress_data(value)?)?;
        }
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?.Show(&toast)
    }

    fn show_setup_inner(env_path: &std::path::Path) -> windows::core::Result<()> {
        let Ok(file_url) = url::Url::from_file_path(env_path) else {
            return Ok(());
        };
        let file_url = escape(file_url.as_str());
        let xml = format!(
            "<toast activationType=\"protocol\" launch=\"{file_url}\"><visual><binding template=\"ToastGeneric\"><text>QShareの初期設定が必要です</text><text>.env を編集して QSHARE_TOKEN を設定してください</text></binding></visual><actions><action content=\"編集する\" activationType=\"protocol\" arguments=\"{file_url}\"/></actions></toast>"
        );
        let document = XmlDocument::new()?;
        document.LoadXml(&HSTRING::from(xml))?;
        let toast = ToastNotification::CreateToastNotification(&document)?;
        toast.SetTag(&tag())?;
        toast.SetGroup(&group())?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?.Show(&toast)
    }

    fn update_progress(value: u8) -> windows::core::Result<NotificationUpdateResult> {
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?
            .UpdateWithTagAndGroup(&progress_data(value)?, &tag(), &group())
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
            &HSTRING::from(format!("{value}%")),
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
