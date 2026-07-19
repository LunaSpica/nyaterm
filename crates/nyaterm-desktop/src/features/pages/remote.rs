use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, MouseButton, ScrollDelta,
    ScrollWheelEvent, Window, div, prelude::*, px, rgb, svg,
};

use crate::widgets::{empty_panel, small_button, status_pill, svg_icon_button};
use std::collections::{HashMap, HashSet};

use super::super::{
    DockerConfirmAction, DockerConfirmState, DockerTab, NyaTermApp,
    RemoteProcessSignalConfirmState, RemoteProcessSortDirection, RemoteProcessSortKey, compact_id,
    docker_compose_project_key, docker_state_color, docker_state_label, docker_state_rank,
    format_file_size, format_rate, format_uptime, metric, modal_dialog_shell, stats_progress_bar,
    transfer_input,
};
use nyaterm_core::truncate_preview;
use nyaterm_transport::{
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
