/// Tauri `resolveConnectionIcon`: map stored icon key / connection kind to SVG path + color.
#[derive(Debug, Clone, Copy)]
pub(in crate::features) struct ConnectionIconDef {
    pub path: &'static str,
    pub color: u32,
    pub glyph: &'static str,
}

/// Keys exposed by the Tauri connection icon picker, in its display order.
pub(in crate::features) const CONNECTION_ICON_OPTIONS: &[&str] = &[
    "server",
    "server-emerald",
    "server-amber",
    "server-rose",
    "server-violet",
    "server-cyan",
    "server-slate",
    "windows",
    "apple",
    "android",
    "linux",
    "ubuntu",
    "debian",
    "centos",
    "fedora",
    "arch",
    "manjaro",
    "opensuse",
    "rocky",
    "alma",
    "alpine",
    "kali",
    "mint",
    "nixos",
    "gentoo",
    "freebsd",
    "raspberrypi",
];

pub(in crate::features) fn resolve_connection_icon(
    icon_key: Option<&str>,
    kind: &str,
) -> ConnectionIconDef {
    if let Some(key) = icon_key.map(str::trim).filter(|value| !value.is_empty()) {
        if let Some(def) = connection_icon_by_key(key) {
            return def;
        }
    }
    default_connection_icon_for_kind(kind)
}

fn connection_icon_by_key(key: &str) -> Option<ConnectionIconDef> {
    let key = key.to_ascii_lowercase();
    // Server palette (Tauri SERVER_ICONS colors).
    let server = match key.as_str() {
        "server" => Some(0x60a5fa),
        "server-emerald" => Some(0x34d399),
        "server-amber" => Some(0xfbbf24),
        "server-rose" => Some(0xfb7185),
        "server-violet" => Some(0xa78bfa),
        "server-cyan" => Some(0x22d3ee),
        "server-slate" => Some(0x94a3b8),
        _ => None,
    };
    if let Some(color) = server {
        return Some(ConnectionIconDef {
            path: "icons/conn/server.svg",
            color,
            glyph: "☰",
        });
    }
    Some(match key.as_str() {
        "linux" => ConnectionIconDef {
            path: "icons/conn/linux.svg",
            color: 0xfcc624,
            glyph: "🐧",
        },
        "ubuntu" => ConnectionIconDef {
            path: "icons/conn/ubuntu.svg",
            color: 0xe95420,
            glyph: "U",
        },
        "debian" => ConnectionIconDef {
            path: "icons/conn/debian.svg",
            color: 0xa81d33,
            glyph: "D",
        },
        "centos" | "fedora" | "arch" | "manjaro" | "opensuse" | "rocky" | "alma" | "alpine"
        | "kali" | "mint" | "nixos" | "gentoo" | "freebsd" | "raspberrypi" => ConnectionIconDef {
            path: "icons/conn/linux.svg",
            color: match key.as_str() {
                "centos" => 0xa14f8c,
                "fedora" => 0x3c4fb1,
                "arch" => 0x1793d1,
                "manjaro" => 0x35bf5c,
                "opensuse" => 0x73ba25,
                "rocky" => 0x10b981,
                "alma" => 0xff4649,
                "alpine" => 0x0d597f,
                "kali" => 0x268bee,
                "mint" => 0x87cf3e,
                "nixos" => 0x5277c3,
                "gentoo" => 0x54487a,
                "freebsd" => 0xab2b28,
                "raspberrypi" => 0xa22846,
                _ => 0xfcc624,
            },
            glyph: "🐧",
        },
        "apple" => ConnectionIconDef {
            path: "icons/conn/apple.svg",
            color: 0xa2aaad,
            glyph: "",
        },
        "windows" => ConnectionIconDef {
            path: "icons/conn/windows.svg",
            color: 0x0078d4,
            glyph: "▣",
        },
        "android" => ConnectionIconDef {
            path: "icons/conn/linux.svg",
            color: 0x3ddc84,
            glyph: "A",
        },
        "docker" => ConnectionIconDef {
            path: "icons/conn/docker.svg",
            color: 0x2496ed,
            glyph: "🐋",
        },
        "python" => ConnectionIconDef {
            path: "icons/conn/python.svg",
            color: 0x3776ab,
            glyph: "Py",
        },
        "github" => ConnectionIconDef {
            path: "icons/conn/github.svg",
            color: 0xc9d1d9,
            glyph: "GH",
        },
        "k8s" | "kubernetes" => ConnectionIconDef {
            path: "icons/conn/docker.svg",
            color: 0x326ce5,
            glyph: "K",
        },
        "local" | "terminal" => ConnectionIconDef {
            path: "icons/conn/terminal.svg",
            color: 0x4ade80,
            glyph: ">_",
        },
        "telnet" => ConnectionIconDef {
            path: "icons/conn/telnet.svg",
            color: 0xd29922,
            glyph: "⇄",
        },
        "serial" => ConnectionIconDef {
            path: "icons/conn/serial.svg",
            color: 0xbc8cff,
            glyph: "⌁",
        },
        "folder" | "group" => ConnectionIconDef {
            path: "icons/conn/folder.svg",
            color: 0xfbbf24,
            glyph: "📁",
        },
        // Other QUICK_ICONS keys fall back to colored server glyph.
        "nginx" | "redis" | "postgres" | "mysql" | "mongodb" | "js" | "ts" | "rust" | "go"
        | "node" | "php" | "aws" | "gcp" | "gitlab" => ConnectionIconDef {
            path: "icons/conn/server.svg",
            color: match key.as_str() {
                "nginx" => 0x009639,
                "redis" => 0xdc382d,
                "postgres" => 0x4169e1,
                "mysql" => 0x4479a1,
                "mongodb" => 0x47a248,
                "js" => 0xf7df1e,
                "ts" => 0x3178c6,
                "rust" => 0xdea584,
                "go" => 0x00add8,
                "node" => 0x339933,
                "php" => 0x777bb4,
                "aws" => 0xff9900,
                "gcp" => 0x4285f4,
                "gitlab" => 0xfc6d26,
                _ => 0x60a5fa,
            },
            glyph: "☰",
        },
        _ => return None,
    })
}

fn default_connection_icon_for_kind(kind: &str) -> ConnectionIconDef {
    match kind {
        "Local" => ConnectionIconDef {
            path: "icons/conn/terminal.svg",
            color: 0x4ade80,
            glyph: ">_",
        },
        "Telnet" => ConnectionIconDef {
            path: "icons/conn/telnet.svg",
            color: 0xd29922,
            glyph: "⇄",
        },
        "Serial" => ConnectionIconDef {
            path: "icons/conn/serial.svg",
            color: 0xbc8cff,
            glyph: "⌁",
        },
        _ => ConnectionIconDef {
            path: "icons/conn/server.svg",
            color: 0x60a5fa,
            glyph: "☰",
        },
    }
}
