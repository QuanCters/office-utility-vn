use super::{Json, SaveAnswer, SheetApp, json};

impl SheetApp {
    /// Holds back a window close (the close button, Alt+F4, the OS quitting the app) while a
    /// workbook has unsaved changes, and asks about them first.
    pub(super) fn guard_window_close(&mut self, ctx: &egui::Context) {
        if self.native_windows || self.allow_close || !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        // A cell still being edited is part of the workbook, as for File › Close.
        if !self.commit_edit(0, 0, false, false) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }
        // The macro step editor asks about its own unsaved steps and debug run first; once it has
        // closed, the window close goes on.
        if self.dialog.as_ref().is_some_and(|d| d.name == "macroEditor") {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            crate::macro_editor::request_close(self, true);
            return;
        }
        if !self.session.documents().iter().any(|d| d.is_dirty()) {
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        match self.dialog.as_mut() {
            // Already asking about a workbook (File › Close): answering it now goes on to quit.
            Some(d) if d.name() == "saveChanges" => {
                d.values.insert("quit".into(), json!(true));
            }
            _ => self.continue_quit(ctx),
        }
    }

    /// Asks about the next unsaved workbook before quitting, in its window, or closes the app
    /// once none is left.
    pub(crate) fn continue_quit(&mut self, ctx: &egui::Context) {
        let dirty = self.session.documents().iter().find(|d| d.is_dirty()).map(|d| d.uid);
        match dirty.and_then(|uid| self.session.doc_window(uid)) {
            Some(window) => {
                // Its window comes forward (the native window too: see `doc_windows`).
                self.enter_window(window);
                let title = self.session.active().map(|d| d.display_title()).unwrap_or_default();
                self.open_dialog("saveChanges", json!({"title": title, "quit": true}));
                self.bring_forward = true;
            }
            None => {
                self.allow_close = true;
                if let Some(cb) = &self.services.close_confirmed {
                    cb();
                }
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
            }
        }
    }

    /// Answers the Save Changes question about the active workbook (its buttons). While quitting,
    /// the next unsaved workbook is asked about once this one is answered; Cancel stops the quit.
    /// Returns whether the question is settled: Save keeps it open when the cell being edited
    /// can't be entered.
    pub fn answer_save_changes(&mut self, answer: SaveAnswer, quitting: bool, ctx: &egui::Context) -> bool {
        self.answer_save_changes_with_next(answer, quitting, false, &json!({}), false, ctx)
    }

    /// Answers the Save Changes question with optional next action (file.new or file.open).
    pub fn answer_save_changes_with_next(
        &mut self,
        answer: SaveAnswer,
        quitting: bool,
        new_after: bool,
        new_params: &Json,
        open_after: bool,
        ctx: &egui::Context,
    ) -> bool {
        match answer {
            SaveAnswer::Cancel => {
                if quitting {
                    if let Some(cb) = &self.services.close_cancelled {
                        cb();
                    }
                }
            }
            SaveAnswer::DontSave => {
                self.run_or_alert("file.close", json!({"force": true, "document": true}));
                if quitting {
                    self.continue_quit(ctx);
                } else if open_after {
                    if let Some(start) = &self.services.open_async {
                        start();
                    }
                } else if new_after || self.session.documents().is_empty() {
                    let mut p = new_params.clone();
                    p["force"] = json!(true);
                    self.run_or_alert("file.new", p);
                }
            }
            SaveAnswer::Save => {
                // File operations include the cell still being edited, as File › Save does.
                if !self.commit_edit(0, 0, false, false) {
                    return false;
                }
                if new_after || open_after || quitting {
                    if let Some(doc) = self.session.active() {
                        let intent = if open_after {
                            crate::state::DeferredActionIntent::Open
                        } else if new_after {
                            crate::state::DeferredActionIntent::New { params: new_params.clone() }
                        } else {
                            crate::state::DeferredActionIntent::Close
                        };
                        self.pending_deferred_action = Some(crate::state::PendingDeferredAction {
                            doc_uid: doc.uid,
                            intent,
                            snapshot: doc.wb.clone(),
                        });
                    }
                }
                self.run_or_alert("file.save", json!({}));
                if self.session.active().is_some_and(|x| !x.is_dirty()) {
                    if let Some(action) = self.pending_deferred_action.take() {
                        self.execute_deferred_action(action, ctx);
                    }
                }
            }
        }
        true
    }

    /// Executes an authorized deferred intent (New, Open, Close) after save.
    pub fn execute_deferred_action(&mut self, action: crate::state::PendingDeferredAction, ctx: &egui::Context) {
        if let Some(doc) = self.session.active() {
            if doc.uid == action.doc_uid && !std::sync::Arc::ptr_eq(&doc.wb, &action.snapshot) {
                return;
            }
        }
        match action.intent {
            crate::state::DeferredActionIntent::Close => {
                self.run_or_alert("file.close", json!({"force": true, "document": true}));
                self.continue_quit(ctx);
            }
            crate::state::DeferredActionIntent::Open => {
                self.run_or_alert("file.close", json!({"force": true, "document": true}));
                if let Some(start) = &self.services.open_async {
                    start();
                }
            }
            crate::state::DeferredActionIntent::New { params } => {
                self.run_or_alert("file.close", json!({"force": true, "document": true}));
                let mut p = params.clone();
                p["force"] = json!(true);
                self.run_or_alert("file.new", p);
            }
        }
    }

    /// Authorizes and executes pending deferred action when native persistence confirms.
    pub fn on_save_confirmed(&mut self, doc_uid: u64, snapshot: &std::sync::Arc<gridcraft_engine::model::Workbook>, ctx: &egui::Context) {
        if let Some(action) = self.pending_deferred_action.take() {
            if action.doc_uid == doc_uid {
                if let Some(active_doc) = self.session.active() {
                    if active_doc.uid == doc_uid && std::sync::Arc::ptr_eq(&active_doc.wb, snapshot) {
                        self.execute_deferred_action(action, ctx);
                    }
                }
            }
        }
    }

    /// Quit (the macOS app menu, ⌘Q): commits every window's cell draft as Enter would, lets the
    /// macro step editor ask about its unsaved steps, then asks about each unsaved workbook in its
    /// window; Cancel on any of them stops the quit. A draft its
    /// workbook refuses stops the quit too, with its window in front and the reason shown.
    pub fn request_quit(&mut self, ctx: &egui::Context) {
        while let Some(window) = self.parked_window_with_draft() {
            self.enter_window(window);
            if self.settle_drafts().is_err() {
                return;
            }
        }
        if self.settle_drafts().is_err() {
            return;
        }
        // The macro step editor asks about its own unsaved steps and debug run first, as for the
        // window's close button, in the window it is shown in; once it has closed, the quit goes
        // on, and Cancel stops it.
        if let Some(window) = self.window_with_dialog("macroEditor") {
            if self.ui_window() != Some(window) {
                self.enter_window(window);
            }
            crate::macro_editor::request_close(self, true);
            return;
        }
        if let Some(d) = self.dialog.as_mut().filter(|d| d.name() == "saveChanges") {
            // Already asking about a workbook (File › Close): answering it goes on to quit.
            d.values.insert("quit".into(), json!(true));
            return;
        }
        self.continue_quit(ctx);
    }
}
