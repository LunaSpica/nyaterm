use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, ScrollDelta, ScrollWheelEvent,
    Window, div, prelude::*, px, rgb, svg,
};

use crate::ui::components::{
    capability_line, empty_panel, icon_button, section_header, small_button, status_pill,
};
use std::collections::{HashMap, HashSet};

use super::super::{
    DockerConfirmAction, DockerConfirmState, DockerTab, NyaTermApp,
    RemoteProcessSignalConfirmState, RemoteProcessSortDirection, RemoteProcessSortKey, compact_id,
    docker_compose_project_key, docker_state_color, docker_state_label, docker_state_rank,
    format_file_size, format_rate, format_uptime, metric, stats_progress_bar, stats_resource_row,
    transfer_input,
};
use nyaterm_domain::truncate_preview;
use nyaterm_session::{
    DockerComposeProject, DockerComposeService, DockerContainer, DockerContainerDetails,
    DockerImage, DockerNetwork, DockerVolume, RemoteProcess,
};

#[path = "remote/docker.rs"]
mod docker;
#[path = "remote/docker_view.rs"]
mod docker_view;
#[path = "remote/process.rs"]
mod process;
#[path = "remote/process_view.rs"]
mod process_view;
#[path = "remote/stats_view.rs"]
mod stats_view;


use docker::*;
use process::*;
