pub fn classify(app_name: &str, title: &str, process_path: &str) -> String {
    let app = app_name.to_lowercase();
    let title = title.to_lowercase();
    let path = process_path.to_lowercase();

    if app.contains("pctime") {
        return "PCTime".to_string();
    }

    if contains_any(
        &app,
        &[
            "code",
            "codex",
            "cursor",
            "windsurf",
            "devenv",
            "visual studio",
            "rustrover",
            "webstorm",
            "phpstorm",
            "pycharm",
            "intellij",
            "idea",
            "goland",
            "rider",
            "clion",
            "android studio",
            "godot",
            "unity",
            "unreal",
            "terminal",
            "powershell",
            "wezterm",
            "alacritty",
            "git",
            "postman",
            "insomnia",
            "dbeaver",
            "datagrip",
            "navicat",
            "docker",
            "sourcetree",
            "fork",
            "sublime",
            "notepad++",
            "vim",
            "nvim",
        ],
    ) || app == "wt.exe"
        || contains_any(
            &title,
            &[
                "github",
                "gitlab",
                "stack overflow",
                "docs.rs",
                "developer",
                "godot engine",
                "localhost",
                "127.0.0.1",
                "api reference",
            ],
        )
    {
        return "Development".to_string();
    }

    if contains_any(
        &title,
        &[
            "chatgpt",
            "claude",
            "gemini",
            "perplexity",
            "copilot",
            "通义",
            "kimi",
            "deepseek",
            "豆包",
        ],
    ) || contains_any(&app, &["chatgpt", "claude", "copilot", "perplexity"])
    {
        return "AI Work".to_string();
    }

    if contains_any(
        &app,
        &[
            "chrome", "msedge", "firefox", "brave", "vivaldi", "opera", "arc", "browser",
        ],
    ) || path.contains("browser")
    {
        if contains_any(
            &title,
            &[
                "youtube",
                "bilibili",
                "哔哩哔哩",
                "netflix",
                "spotify",
                "twitch",
                "douyin",
                "抖音",
                "iqiyi",
                "youku",
            ],
        ) {
            return "Media".to_string();
        }

        if contains_any(
            &title,
            &[
                "gmail", "outlook", "mail", "slack", "discord", "teams", "telegram",
                "whatsapp", "飞书", "钉钉",
            ],
        ) {
            return "Communication".to_string();
        }

        if contains_any(
            &title,
            &[
                "docs",
                "notion",
                "read",
                "wiki",
                "search",
                "google",
                "baidu",
                "百度",
                "wikipedia",
                "zhihu",
                "知乎",
                "arxiv",
                "paper",
            ],
        ) {
            return "Research".to_string();
        }

        return "Browser".to_string();
    }

    if contains_any(
        &app,
        &[
            "wechat", "weixin", "qq", "dingtalk", "lark", "feishu", "slack", "discord",
            "telegram", "teams", "zoom",
        ],
    ) || contains_any(
        &title,
        &[
            "wechat", "weixin", "微信", "qq", "飞书", "钉钉", "slack", "discord",
            "telegram",
        ],
    ) {
        return "Communication".to_string();
    }

    if contains_any(
        &app,
        &["zoom", "teams", "tencentmeeting", "voovmeeting", "meeting"],
    ) || contains_any(&title, &["zoom meeting", "腾讯会议", "会议"])
    {
        return "Meetings".to_string();
    }

    if contains_any(
        &app,
        &[
            "steam",
            "epic",
            "battle.net",
            "riot",
            "minecraft",
            "bg3",
            "genshin",
            "starrail",
            "zzz",
        ],
    ) || contains_any(&title, &["baldur", "game", "vulkan", "directx"])
    {
        return "Games".to_string();
    }

    if contains_any(
        &app,
        &[
            "explorer",
            "taskmgr",
            "settings",
            "control",
            "regedit",
            "mmc",
            "cmd",
            "conhost",
            "services",
            "powershell_ise",
        ],
    ) {
        return "System".to_string();
    }

    if contains_any(
        &app,
        &[
            "excel",
            "word",
            "powerpnt",
            "wps",
            "notepad",
            "obsidian",
            "onenote",
            "typora",
            "zotero",
            "acrobat",
            "foxit",
            "sumatrapdf",
        ],
    ) || contains_any(&title, &["notion", "文档", "pdf"])
    {
        return "Documents".to_string();
    }

    if contains_any(
        &app,
        &[
            "figma",
            "photoshop",
            "illustrator",
            "indesign",
            "xd",
            "sketch",
            "affinity",
            "canva",
            "clipstudio",
        ],
    ) {
        return "Design".to_string();
    }

    if contains_any(
        &app,
        &[
            "blender",
            "maya",
            "houdini",
            "cinema4d",
            "c4d",
            "3dsmax",
            "zbrush",
            "substance",
            "krita",
            "davinci",
            "resolve",
            "premiere",
            "afterfx",
            "audition",
            "obs64",
            "obs.exe",
        ],
    ) {
        return "Creative".to_string();
    }

    if contains_any(
        &app,
        &[
            "thunderbird",
            "todoist",
            "ticktick",
            "trello",
            "asana",
            "linear",
            "monday",
            "calendar",
        ],
    ) {
        return "Productivity".to_string();
    }

    if contains_any(
        &app,
        &["onedrive", "dropbox", "googledrive", "synology", "nas", "winscp"],
    ) {
        return "Cloud".to_string();
    }

    if contains_any(
        &app,
        &["binance", "tradingview", "metatrader", "futu", "moomoo", "富途"],
    ) || contains_any(&title, &["tradingview", "股票", "期货", "crypto", "bitcoin"])
    {
        return "Finance".to_string();
    }

    if contains_any(
        &app,
        &["7z", "winrar", "everything", "powertoys", "snipaste", "sharex"],
    ) {
        return "Utilities".to_string();
    }

    "Unclassified".to_string()
}

fn contains_any(value: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| value.contains(pattern))
}
