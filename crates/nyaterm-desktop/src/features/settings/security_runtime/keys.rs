use gpui::{Context, KeyDownEvent, PathPromptOptions, SharedString, Window};
use nyaterm_core::{ConnectionStore, SshKey};

use crate::features::{NyaTermApp, compact_id};
use crate::models::{
    SecurityAuthTab, SecurityDeleteConfirmState, SecurityKeyEditorField, SecurityKeyEditorState,
};

impl NyaTermApp {
    pub(in crate::features) fn open_security_key_editor(
        &mut self,
        key_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.forget_text_inputs("security.editor.key-");
        let editor = if let Some(key_id) = key_id {
            let Some(key) = self
                .security
                .ssh_keys()
                .iter()
                .find(|key| key.id == key_id)
                .cloned()
            else {
                self.security.set_status("SSH key is no longer available");
                cx.notify();
                return;
            };
            SecurityKeyEditorState {
                id: Some(key.id),
                name: key.name,
                key_file_path: String::new(),
                cert_file_path: String::new(),
                passphrase: String::new(),
                has_key_data: key.has_key_data,
                has_cert_data: key.has_cert_data,
                focused_field: SecurityKeyEditorField::Name,
                error: None,
            }
        } else {
            SecurityKeyEditorState {
                id: None,
                name: String::new(),
                key_file_path: String::new(),
                cert_file_path: String::new(),
                passphrase: String::new(),
                has_key_data: false,
                has_cert_data: false,
                focused_field: SecurityKeyEditorField::Name,
                error: None,
            }
        };
        self.security
            .open_key_editor(editor, "SSH key editor opened".to_string());
        window.focus(self.security.key_editor_focus());
        cx.notify();
    }

    pub(in crate::features) fn close_security_key_editor(&mut self, cx: &mut Context<Self>) {
        self.forget_text_inputs("security.editor.key-");
        self.security.close_key_editor();
        cx.notify();
    }

    pub(in crate::features) fn focus_security_key_field(
        &mut self,
        field: SecurityKeyEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security.key_editor_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(self.security.key_editor_focus());
        cx.notify();
    }

    pub(in crate::features) fn handle_security_key_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        // The boxes own the text; the editor owns the keys that close or save
        // it, which the boxes leave unconsumed.
        match keystroke.key.as_str() {
            "escape" => {
                self.close_security_key_editor(cx);
                return;
            }
            "enter" => {
                self.save_security_key_editor(window, cx);
                return;
            }
            _ => {}
        }
    }

    pub(in crate::features) fn save_security_key_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.security.key_editor().cloned() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            if let Some(editor) = self.security.key_editor_mut() {
                editor.error = Some("key name is required".to_string());
            }
            cx.notify();
            return;
        }
        if editor.id.is_none() && editor.key_file_path.trim().is_empty() && !editor.has_key_data {
            if let Some(editor) = self.security.key_editor_mut() {
                editor.error = Some("select a private key file".to_string());
            }
            cx.notify();
            return;
        }

        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                if let Some(editor) = self.security.key_editor_mut() {
                    editor.error = Some(error.to_string());
                }
                cx.notify();
                return;
            }
        };

        let key = SshKey {
            id: editor.id.clone().unwrap_or_default(),
            name,
            key: None,
            cert: None,
            passphrase: if editor.passphrase.trim().is_empty() {
                None
            } else {
                Some(editor.passphrase.clone())
            },
            key_file_path: if editor.key_file_path.trim().is_empty() {
                None
            } else {
                Some(editor.key_file_path.trim().to_string())
            },
            cert_file_path: if editor.cert_file_path.trim().is_empty() {
                None
            } else {
                Some(editor.cert_file_path.trim().to_string())
            },
            has_key_data: false,
            has_cert_data: false,
        };

        match store.save_ssh_key(key) {
            Ok(id) => {
                self.refresh_security_catalog();
                self.security
                    .finish_key_editor(format!("SSH key saved ({})", compact_id(&id)));
                self.shell.status = "SSH key saved".to_string();
            }
            Err(error) => {
                if let Some(editor) = self.security.key_editor_mut() {
                    editor.error = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn request_delete_security_key(
        &mut self,
        key_id: String,
        cx: &mut Context<Self>,
    ) {
        let label = self
            .security
            .ssh_keys()
            .iter()
            .find(|key| key.id == key_id)
            .map(|key| key.name.clone())
            .unwrap_or_else(|| key_id.clone());
        self.security.request_delete(SecurityDeleteConfirmState {
            kind: SecurityAuthTab::Keys,
            id: key_id,
            label,
        });
        cx.notify();
    }

    pub(in crate::features) fn pick_security_key_file(
        &mut self,
        is_cert: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(if is_cert {
                "Select certificate file"
            } else {
                "Select private key file"
            })),
        };
        let receiver = cx.prompt_for_paths(options);
        self.security.set_status(if is_cert {
            "selecting certificate file"
        } else {
            "selecting private key file"
        });
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    let path = path.display().to_string();
                    if let Some(editor) = this.security.key_editor_mut() {
                        if is_cert {
                            editor.cert_file_path = path;
                            editor.has_cert_data = true;
                        } else {
                            editor.key_file_path = path;
                            editor.has_key_data = true;
                        }
                        editor.error = None;
                        this.security.set_status("key file selected");
                    }
                } else {
                    this.security.set_status("key file selection cancelled");
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
