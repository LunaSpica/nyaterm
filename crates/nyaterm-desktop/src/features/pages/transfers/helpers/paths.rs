use super::*;

pub(in crate::features::pages::transfers) fn remote_file_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

pub(in crate::features::pages::transfers) fn remote_parent_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    match path.rfind('/') {
        Some(0) => "/".to_string(),
        Some(index) => path[..index].to_string(),
        None => ".".to_string(),
    }
}

pub(in crate::features::pages::transfers) fn remote_sibling_path(
    old_path: &str,
    new_name: &str,
) -> String {
    match remote_parent_path(old_path).as_str() {
        "/" => format!("/{new_name}"),
        "." => new_name.to_string(),
        parent => format!("{parent}/{new_name}"),
    }
}

pub(in crate::features::pages::transfers) fn remote_child_path(
    parent: &str,
    child_name: &str,
) -> String {
    match parent.trim_end_matches('/') {
        "" | "." => child_name.to_string(),
        "/" => format!("/{child_name}"),
        parent => format!("{parent}/{child_name}"),
    }
}

pub(in crate::features::pages::transfers) fn normalized_transfer_browser_path(
    path: &str,
) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        ".".to_string()
    } else if trimmed == "/" {
        "/".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

pub(in crate::features::pages::transfers) fn valid_remote_child_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/')
}

#[derive(Debug, Clone, Copy)]
pub(in crate::features::pages::transfers) enum TransferPathPart {
    Full,
    Name,
    Directory,
}

impl TransferPathPart {
    pub(in crate::features::pages::transfers) fn label(self) -> &'static str {
        match self {
            Self::Full => "path",
            Self::Name => "name",
            Self::Directory => "directory",
        }
    }
}

pub(in crate::features::pages::transfers) const TRANSFER_BROWSER_ACTIONS_WIDTH: gpui::Pixels =
    px(44.);
const TRANSFER_BROWSER_COLUMN_GAP_TOTAL: gpui::Pixels = px(48.);

pub(in crate::features::pages::transfers) fn transfer_browser_table_width(
    widths: TransferBrowserColumnWidths,
) -> gpui::Pixels {
    widths.name
        + widths.modified
        + widths.size
        + widths.permissions
        + widths.owner
        + widths.group
        + TRANSFER_BROWSER_ACTIONS_WIDTH
        + TRANSFER_BROWSER_COLUMN_GAP_TOTAL
}

pub(in crate::features::pages::transfers) fn transfer_path_part_value(
    path: &str,
    part: TransferPathPart,
) -> String {
    match part {
        TransferPathPart::Full => path.to_string(),
        TransferPathPart::Name => remote_file_name(path),
        TransferPathPart::Directory => remote_parent_path(path),
    }
}

pub(in crate::features::pages::transfers) fn format_sftp_modified(value: Option<u32>) -> String {
    value
        .map(|seconds| format!("{seconds}s"))
        .unwrap_or_else(|| "-".to_string())
}
