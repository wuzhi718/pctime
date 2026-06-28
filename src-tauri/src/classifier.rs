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
            "code - insiders",
            "vscodium",
            "zed",
            "codex",
            "cursor",
            "windsurf",
            "trae",
            "kiro",
            "devenv",
            "visual studio",
            "eclipse",
            "netbeans",
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
            "pgadmin",
            "mysqlworkbench",
            "redisinsight",
            "mongodb compass",
            "docker",
            "sourcetree",
            "fork",
            "githubdesktop",
            "github desktop",
            "sublime",
            "notepad++",
            "vim",
            "nvim",
            "mobaxterm",
            "xshell",
            "putty",
            "cmder",
            "tabby",
            "warp",
            "ubuntu",
            "wsl",
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
                "devdocs",
                "mdn",
                "docker hub",
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
            "poe",
            "monica",
            "秘塔",
            "metaso",
            "天工",
        ],
    ) || contains_any(
        &app,
        &[
            "chatgpt",
            "claude",
            "copilot",
            "perplexity",
            "poe",
            "lm studio",
            "lmstudio",
            "ollama",
            "jan",
            "cherry studio",
            "chatbox",
        ],
    ) {
        return "AI Work".to_string();
    }

    if contains_any(
        &app,
        &[
            "chrome", "chromium", "msedge", "firefox", "floorp", "waterfox", "brave", "vivaldi",
            "opera", "arc", "yandex", "thorium", "browser",
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
                "spotify",
                "music",
            ],
        ) {
            return "Media".to_string();
        }

        if contains_any(
            &title,
            &[
                "gmail", "outlook", "mail", "slack", "discord", "teams", "telegram", "whatsapp",
                "飞书", "钉钉",
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
                "semantic scholar",
                "researchgate",
                "pubmed",
            ],
        ) {
            return "Research".to_string();
        }

        return "Browser".to_string();
    }

    if contains_any(
        &app,
        &[
            "vlc",
            "potplayer",
            "mpv",
            "foobar",
            "musicbee",
            "spotify",
            "qqmusic",
            "cloudmusic",
            "netease",
            "kugou",
            "bilibili",
            "douyin",
            "twitch",
            "plex",
            "jellyfin",
        ],
    ) {
        return "Media".to_string();
    }

    if contains_any(
        &app,
        &[
            "wechat",
            "weixin",
            "qq",
            "dingtalk",
            "lark",
            "feishu",
            "slack",
            "discord",
            "telegram",
            "teams",
            "outlook",
            "mattermost",
            "signal",
            "element",
            "line",
            "kakaotalk",
            "dingding",
        ],
    ) || contains_any(
        &title,
        &[
            "wechat",
            "weixin",
            "微信",
            "qq",
            "飞书",
            "钉钉",
            "slack",
            "discord",
            "telegram",
            "mattermost",
            "signal",
            "whatsapp",
        ],
    ) {
        return "Communication".to_string();
    }

    if contains_any(
        &app,
        &[
            "zoom",
            "teams",
            "tencentmeeting",
            "voovmeeting",
            "feishu meeting",
            "lark meeting",
            "meeting",
            "webex",
            "gotomeeting",
        ],
    ) || contains_any(&title, &["zoom meeting", "腾讯会议", "飞书会议", "会议"])
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
            "leagueclient",
            "valorant",
            "ea app",
            "eadesktop",
            "ubisoft",
            "gog galaxy",
            "rockstar",
            "minecraft",
            "roblox",
            "bg3",
            "genshin",
            "starrail",
            "zzz",
            "dota",
            "cs2",
            "overwatch",
            "hearthstone",
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
            "msconfig",
            "eventvwr",
            "perfmon",
            "resmon",
            "devmgmt",
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
            "libreoffice",
            "soffice",
            "wps",
            "notepad",
            "notion",
            "obsidian",
            "onenote",
            "typora",
            "logseq",
            "siyuan",
            "yuque",
            "zotero",
            "calibre",
            "xmind",
            "mindmanager",
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
            "lunacy",
            "axure",
            "balsamiq",
            "zeplin",
            "draw.io",
            "drawio",
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
            "vlc",
            "potplayer",
            "mpv",
            "foobar",
            "musicbee",
            "spotify",
            "qqmusic",
            "cloudmusic",
            "netease",
            "kugou",
            "ableton",
            "fl studio",
            "fl64",
            "reaper",
            "cubase",
            "aseprite",
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
            "superhuman",
            "notion calendar",
            "滴答清单",
        ],
    ) {
        return "Productivity".to_string();
    }

    if contains_any(
        &app,
        &[
            "onedrive",
            "dropbox",
            "googledrive",
            "google drive",
            "synology",
            "nas",
            "winscp",
            "filezilla",
            "baidunetdisk",
            "百度网盘",
            "aliyundrive",
            "阿里云盘",
            "quark",
            "115",
            "nutstore",
            "坚果云",
            "mega",
            "terabox",
        ],
    ) {
        return "Cloud".to_string();
    }

    if contains_any(
        &app,
        &[
            "binance",
            "tradingview",
            "metatrader",
            "mt4",
            "mt5",
            "futu",
            "moomoo",
            "富途",
            "同花顺",
            "雪球",
            "eastmoney",
            "东方财富",
        ],
    ) || contains_any(
        &title,
        &["tradingview", "股票", "期货", "crypto", "bitcoin"],
    ) {
        return "Finance".to_string();
    }

    if contains_any(
        &app,
        &[
            "7z",
            "winrar",
            "bandizip",
            "everything",
            "powertoys",
            "snipaste",
            "sharex",
            "utools",
            "listary",
            "ditto",
            "quicklook",
            "eartrumpet",
            "trafficmonitor",
            "processhacker",
            "procexp",
            "autoruns",
            "hwinfo",
            "cpu-z",
            "gpu-z",
            "rufus",
            "wiztree",
            "spacesniffer",
        ],
    ) {
        return "Utilities".to_string();
    }

    "Unclassified".to_string()
}

fn contains_any(value: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| value.contains(pattern))
}
