use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, MouseButton, ScrollDelta, ScrollWheelEvent,
    Window, div, prelude::*, px, rgb, svg,
};

use crate::models::{DockerConfirmAction, DockerConfirmState, DockerTab};
use crate::widgets::{empty_panel, small_button, status_pill, svg_icon_button};
use std::collections::{HashMap, HashSet};

use super::super::{
    NyaTermApp, TextInputSetup, compact_id, docker_compose_project_key, docker_state_color,
    docker_state_label, docker_state_rank, format_file_size, format_rate, format_uptime,
    modal_dialog_shell, stats_progress_bar,
};
use nyaterm_core::truncate_preview;
use nyaterm_transport::{
    DockerComposeProject, DockerComposeService, DockerContainer, DockerContainerDetails,
    DockerImage, DockerNetwork, DockerVolume,
};

mod docker;
mod docker_view;
mod process;
mod process_view;
mod stats_view;

use docker::*;
use process::*;
