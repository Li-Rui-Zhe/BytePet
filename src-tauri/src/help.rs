use std::process::Command;

const LINKS: &str = include_str!("../../src/config/deepseekHelpLinks.json");

fn resolve_link(id: &str) -> Result<String, String> {
    let links: serde_json::Value = serde_json::from_str(LINKS).map_err(|_| "帮助链接配置损坏。")?;
    let url = links
        .get(id)
        .and_then(|item| item.get("url"))
        .and_then(|url| url.as_str())
        .ok_or("不支持的帮助链接。")?;
    let parsed = reqwest::Url::parse(url).map_err(|_| "帮助链接地址无效。")?;
    if parsed.scheme() != "https"
        || !matches!(
            parsed.host_str(),
            Some("platform.deepseek.com" | "api-docs.deepseek.com")
        )
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("只能打开 DeepSeek 官方帮助页面。".into());
    }
    Ok(url.into())
}

#[tauri::command]
pub fn open_deepseek_help_link(id: String) -> Result<(), String> {
    let url = resolve_link(&id)?;
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let root = std::env::var_os("SystemRoot").ok_or("无法定位系统浏览器启动程序。")?;
        let mut command =
            Command::new(std::path::PathBuf::from(root).join("System32/rundll32.exe"));
        command
            .arg("url.dll,FileProtocolHandler")
            .creation_flags(0x08000000);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = Command::new("/usr/bin/open");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = Command::new("/usr/bin/xdg-open");
    command
        .arg(url)
        .spawn()
        .map_err(|_| "无法打开浏览器，请按教程中的网址手动访问。".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opens_only_bundled_deepseek_destinations() {
        for id in ["keys", "balance", "docs", "pricing", "errors"] {
            assert!(resolve_link(id).unwrap().starts_with("https://"));
        }
        for id in [
            "https://evil.example",
            "file:///secret",
            "../keys",
            "keys?url=evil",
            "",
        ] {
            assert!(resolve_link(id).is_err());
        }
    }
}
