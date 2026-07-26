use super::*;
use crate::models::QuickCommandImportPathPromptKind;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;

#[derive(Debug, Default)]
struct ImportSummary {
    imported_commands: usize,
    imported_categories: usize,
    updated_commands: usize,
    total_commands: usize,
    total_categories: usize,
}

#[derive(Debug, Default)]
struct ImportConfig {
    commands: Vec<ImportCommand>,
    categories: Vec<ImportCategory>,
}

#[derive(Debug)]
struct ImportCategory {
    id: Option<String>,
    name: String,
}

#[derive(Debug)]
struct ImportCommand {
    id: Option<String>,
    label: String,
    command: String,
    category_id: Option<String>,
    category: Option<String>,
    description: Option<String>,
    color_tag: Option<String>,
    icon_tag: Option<String>,
    pinned: Option<bool>,
    execution_mode: Option<String>,
    source: Option<String>,
    risk_level: Option<RiskLevel>,
}

impl QuickCommandImportPathPromptKind {
    fn prompt_label(self) -> &'static str {
        match self {
            Self::NyatermJson => "Import NyaTerm quick commands JSON",
            Self::WindTermQuickbar => "Import WindTerm quickbar.config",
            Self::XshellXts => "Import Xshell quick buttons .xts",
        }
    }

    fn selecting_status(self) -> &'static str {
        match self {
            Self::NyatermJson => "selecting quick command JSON import file",
            Self::WindTermQuickbar => "selecting WindTerm quickbar import file",
            Self::XshellXts => "selecting Xshell quick button import file",
        }
    }
}

#[path = "import/dialog.rs"]
mod dialog;
#[path = "import/helpers.rs"]
mod helpers;
#[path = "import/json.rs"]
mod json;
#[path = "import/merge.rs"]
mod merge;
#[path = "import/sources.rs"]
mod sources;

use helpers::*;
use json::*;
use merge::*;
use sources::*;

#[cfg(test)]
#[path = "import/tests.rs"]
mod tests;
