use std::env;
#[cfg(not(target_os = "windows"))]
use std::fs;
#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;

pub fn launch_command() -> String {
    let exe = env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "dact".into());
    format!("\"{exe}\" --quiet")
}

pub fn enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_enabled()
    }
    #[cfg(target_os = "macos")]
    {
        plist_path().is_file()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        desktop_path().is_file()
    }
}

pub fn set_enabled(on: bool) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows_set(on)
    }
    #[cfg(target_os = "macos")]
    {
        macos_set(on)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        linux_set(on)
    }
}

#[cfg(target_os = "windows")]
fn windows_enabled() -> bool {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .and_then(|k| k.get_value::<String, _>("DAct"))
        .is_ok()
}

#[cfg(target_os = "windows")]
fn windows_set(on: bool) -> anyhow::Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")?;
    if on {
        key.set_value("DAct", &launch_command())?;
    } else {
        let _ = key.delete_value("DAct");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/LaunchAgents/com.dact.app.plist")
}

#[cfg(target_os = "macos")]
fn macos_set(on: bool) -> anyhow::Result<()> {
    let path = plist_path();
    if on {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let exe = env::current_exe()?;
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.dact.app</string>
  <key>ProgramArguments</key>
  <array><string>{}</string><string>--quiet</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
            exe.display()
        );
        fs::write(path, body)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn desktop_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("autostart/dact.desktop")
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn linux_set(on: bool) -> anyhow::Result<()> {
    let path = desktop_path();
    if on {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let exe = env::current_exe()?;
        fs::write(
            path,
            format!(
                "[Desktop Entry]\nType=Application\nName=DAct\nExec=\"{}\" --quiet\nX-GNOME-Autostart-enabled=true\n",
                exe.display()
            ),
        )?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
