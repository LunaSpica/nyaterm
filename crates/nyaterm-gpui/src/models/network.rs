#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkTab {
    Tunnels,
    Proxies,
}

impl NetworkTab {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Tunnels => "Tunnels",
            Self::Proxies => "Proxies",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkDeleteConfirmState {
    pub(crate) tab: NetworkTab,
    pub(crate) id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkGroupEditorState {
    pub(crate) tab: NetworkTab,
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkGroupDeleteConfirmState {
    pub(crate) tab: NetworkTab,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkMovePickerState {
    pub(crate) tab: NetworkTab,
    pub(crate) id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkTunnelEditorField {
    Name,
    ListenPort,
    TargetHost,
    TargetPort,
}

impl NetworkTunnelEditorField {
    pub(crate) fn next(self, dynamic: bool) -> Self {
        match self {
            Self::Name => Self::ListenPort,
            Self::ListenPort if dynamic => Self::Name,
            Self::ListenPort => Self::TargetHost,
            Self::TargetHost => Self::TargetPort,
            Self::TargetPort => Self::Name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkTunnelEditorState {
    pub(crate) id: Option<String>,
    pub(crate) is_open: bool,
    pub(crate) name: String,
    pub(crate) tunnel_type: String,
    pub(crate) connection_id: Option<String>,
    pub(crate) listen_port: String,
    pub(crate) target_host: String,
    pub(crate) target_port: String,
    pub(crate) auto_open: bool,
    pub(crate) bind_localhost: bool,
    pub(crate) group_id: Option<String>,
    pub(crate) focused_field: NetworkTunnelEditorField,
    pub(crate) error: Option<String>,
}

impl NetworkTunnelEditorState {
    pub(crate) fn is_dynamic(&self) -> bool {
        self.tunnel_type == "dynamic"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkProxyEditorField {
    Name,
    Host,
    Port,
    Command,
    Username,
    Password,
}

impl NetworkProxyEditorField {
    pub(crate) fn next(self, command: bool) -> Self {
        if command {
            return match self {
                Self::Name => Self::Command,
                Self::Command => Self::Name,
                _ => Self::Name,
            };
        }

        match self {
            Self::Name => Self::Host,
            Self::Host => Self::Port,
            Self::Port => Self::Username,
            Self::Username => Self::Password,
            Self::Password => Self::Name,
            Self::Command => Self::Name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkProxyEditorState {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) protocol: String,
    pub(crate) host: String,
    pub(crate) port: String,
    pub(crate) command: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) existing_password: Option<String>,
    pub(crate) password_id: Option<String>,
    pub(crate) group_id: Option<String>,
    pub(crate) focused_field: NetworkProxyEditorField,
    pub(crate) error: Option<String>,
}

impl NetworkProxyEditorState {
    pub(crate) fn is_proxy_command(&self) -> bool {
        self.protocol == "proxycommand"
    }
}
