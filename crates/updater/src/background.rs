//! Non-blocking wrapper around the updater for interactive frontends.
//! Network and disk work runs on a background thread; results come back over a
//! channel drained by `poll()` (once per frame in the GUI).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::{AppInfo, ExitAction, ReleaseInfo, download_and_apply, fetch_latest, is_skipped};

#[derive(Debug, Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(ReleaseInfo),
    Downloading(ReleaseInfo),
    /// Installed (binary) or staged (MSI); a restart applies it.
    Ready(ReleaseInfo),
    Failed(String),
}

/// Surfaced by `poll()`, typically shown as a notification.
#[derive(Debug)]
pub enum UpdateEvent {
    UpToDate,
    Available(semver::Version),
    Ready,
    Error(String),
}

enum Msg {
    Checked(Result<Option<ReleaseInfo>, String>),
    Installed(Result<Option<PathBuf>, String>),
}

type Fetch = fn(&AppInfo) -> Result<Option<ReleaseInfo>, String>;
type Apply = fn(&AppInfo, &ReleaseInfo) -> Result<Option<PathBuf>, String>;

pub struct Updater {
    pub state: UpdateState,
    /// The banner of the available / ready update was put off ("Later"); it
    /// shows again when the next state comes in.
    pub dismissed: bool,
    pub exit_action: Option<ExitAction>,
    app: &'static AppInfo,
    manual: bool,
    staged_msi: Option<PathBuf>,
    repaint: Arc<dyn Fn() + Send + Sync>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl Updater {
    /// `repaint` is called from the worker thread after each result (the GUI
    /// passes `ctx.request_repaint()`).
    pub fn new(app: &'static AppInfo, repaint: impl Fn() + Send + Sync + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            state: UpdateState::Idle,
            dismissed: false,
            exit_action: None,
            app,
            manual: false,
            staged_msi: None,
            repaint: Arc::new(repaint),
            tx,
            rx,
        }
    }

    pub fn current_version(&self) -> &'static str {
        self.app.version
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            self.state,
            UpdateState::Checking | UpdateState::Downloading(_)
        )
    }

    /// Manual checks report "up to date" and errors, and ignore `skipped_version`.
    pub fn check(&mut self, manual: bool, skipped_version: Option<&str>) {
        self.check_with(manual, skipped_version, fetch_latest);
    }

    fn check_with(&mut self, manual: bool, skipped_version: Option<&str>, fetch: Fetch) {
        if self.is_busy() || matches!(self.state, UpdateState::Ready(_)) {
            return;
        }
        self.manual = manual;
        self.dismissed = false;
        self.state = UpdateState::Checking;
        let skipped = if manual {
            None
        } else {
            skipped_version.map(str::to_owned)
        };
        let (tx, repaint, app) = (self.tx.clone(), self.repaint.clone(), self.app);
        std::thread::spawn(move || {
            let res = fetch(app).map(|r| r.filter(|info| !is_skipped(info, skipped.as_deref())));
            let _ = tx.send(Msg::Checked(res));
            repaint();
        });
    }

    pub fn install(&mut self) {
        self.install_with(download_and_apply);
    }

    fn install_with(&mut self, apply: Apply) {
        let UpdateState::Available(info) = &self.state else {
            return;
        };
        let info = info.clone();
        self.state = UpdateState::Downloading(info.clone());
        let (tx, repaint, app) = (self.tx.clone(), self.repaint.clone(), self.app);
        std::thread::spawn(move || {
            let res = apply(app, &info);
            let _ = tx.send(Msg::Installed(res));
            repaint();
        });
    }

    /// Apply the update when the app exits, then relaunch it.
    pub fn schedule_restart(&mut self) {
        if !matches!(self.state, UpdateState::Ready(_)) {
            return;
        }
        self.exit_action = Some(match &self.staged_msi {
            Some(msi) => ExitAction::RunMsiThenRelaunch(msi.clone()),
            None => ExitAction::Relaunch,
        });
    }

    /// On plain quit, a staged MSI still has to run (a replaced binary doesn't).
    pub fn schedule_on_quit(&mut self) {
        if self.exit_action.is_none()
            && let (UpdateState::Ready(_), Some(msi)) = (&self.state, &self.staged_msi)
        {
            self.exit_action = Some(ExitAction::RunMsi(msi.clone()));
        }
    }

    /// "Later" on an available or ready update: hide its banner; a ready one
    /// applies on quit.
    pub fn later(&mut self) {
        match self.state {
            UpdateState::Available(_) => self.dismissed = true,
            UpdateState::Ready(_) => {
                self.dismissed = true;
                self.schedule_on_quit();
            }
            _ => {}
        }
    }

    /// "Skip this version" on an available update: forget it (back to Idle,
    /// so that nothing advertises it any more); its version, to remember.
    pub fn skip(&mut self) -> Option<String> {
        let UpdateState::Available(info) = &self.state else {
            return None;
        };
        let version = info.version.to_string();
        self.state = UpdateState::Idle;
        self.dismissed = false;
        Some(version)
    }

    pub fn poll(&mut self) -> Option<UpdateEvent> {
        match self.rx.try_recv().ok()? {
            Msg::Checked(Ok(Some(info))) => {
                let v = info.version.clone();
                self.state = UpdateState::Available(info);
                Some(UpdateEvent::Available(v))
            }
            Msg::Checked(Ok(None)) => {
                self.state = UpdateState::UpToDate;
                self.manual.then_some(UpdateEvent::UpToDate)
            }
            Msg::Checked(Err(e)) => {
                log::warn!("update check failed: {e}");
                self.state = UpdateState::Failed(e.clone());
                self.manual.then_some(UpdateEvent::Error(e))
            }
            Msg::Installed(res) => {
                // A result we're no longer waiting for: leave the state alone.
                let UpdateState::Downloading(info) = &self.state else {
                    return None;
                };
                let info = info.clone();
                match res {
                    Ok(staged) => {
                        self.staged_msi = staged;
                        self.state = UpdateState::Ready(info);
                        // Downloaded in the background: announce it.
                        self.dismissed = false;
                        Some(UpdateEvent::Ready)
                    }
                    Err(e) => {
                        self.state = UpdateState::Failed(e.clone());
                        Some(UpdateEvent::Error(e))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{APP, msi_release};

    fn info(version: &str) -> ReleaseInfo {
        msi_release(version)
    }

    fn updater() -> Updater {
        Updater::new(&APP, || {})
    }

    fn wait(u: &mut Updater) -> Option<UpdateEvent> {
        for _ in 0..200 {
            if let Some(ev) = u.poll() {
                return Some(ev);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn check_reports_available_and_skips_skipped_version() {
        let mut u = updater();
        u.check_with(false, None, |_| Ok(Some(msi_release("99.0.0"))));
        assert!(matches!(wait(&mut u), Some(UpdateEvent::Available(_))));
        assert!(matches!(u.state, UpdateState::Available(_)));

        let mut u = updater();
        u.check_with(false, Some("v99.0.0"), |_| Ok(Some(msi_release("99.0.0"))));
        assert!(wait(&mut u).is_none());
        assert!(matches!(u.state, UpdateState::UpToDate));
    }

    #[test]
    fn up_to_date_and_errors_only_reported_when_manual() {
        let mut u = updater();
        u.check_with(false, None, |_| Err("boom".into()));
        assert!(wait(&mut u).is_none());
        assert!(matches!(u.state, UpdateState::Failed(_)));

        let mut u = updater();
        u.check_with(true, None, |_| Ok(None));
        assert!(matches!(u.state, UpdateState::Checking));
        assert!(matches!(wait(&mut u), Some(UpdateEvent::UpToDate)));
        assert!(matches!(u.state, UpdateState::UpToDate));
    }

    #[test]
    fn install_moves_to_ready_and_schedules_restart() {
        let mut u = updater();
        u.state = UpdateState::Available(info("99.0.0"));
        u.install_with(|_, _| Ok(Some("x.msi".into())));
        assert!(matches!(u.state, UpdateState::Downloading(_)));
        assert!(matches!(wait(&mut u), Some(UpdateEvent::Ready)));
        u.schedule_restart();
        assert!(matches!(
            u.exit_action,
            Some(ExitAction::RunMsiThenRelaunch(_))
        ));
    }

    #[test]
    fn install_error_is_reported() {
        let mut u = updater();
        u.state = UpdateState::Available(info("99.0.0"));
        u.install_with(|_, _| Err("disk full".into()));
        assert!(matches!(wait(&mut u), Some(UpdateEvent::Error(e)) if e == "disk full"));
    }

    #[test]
    fn stray_install_result_does_not_clobber_state() {
        let mut u = updater();
        u.state = UpdateState::Available(info("99.0.0"));
        u.tx.send(Msg::Installed(Ok(None))).unwrap();
        assert!(u.poll().is_none());
        assert!(matches!(u.state, UpdateState::Available(_)));
    }

    fn ready(msi: Option<&str>) -> Updater {
        let mut u = updater();
        u.state = UpdateState::Ready(info("99.0.0"));
        u.staged_msi = msi.map(PathBuf::from);
        u
    }

    #[test]
    fn later_hides_the_banner_and_a_staged_msi_installs_on_quit() {
        let mut u = ready(Some("x.msi"));
        u.later();
        assert!(u.dismissed);
        assert_eq!(u.exit_action, Some(ExitAction::RunMsi("x.msi".into())));

        // A replaced binary needs nothing at exit.
        let mut u = ready(None);
        u.later();
        assert!(u.dismissed);
        assert!(u.exit_action.is_none());

        // Nothing to put off otherwise.
        let mut u = updater();
        u.later();
        assert!(!u.dismissed);
    }

    #[test]
    fn skip_forgets_the_available_update() {
        let mut u = updater();
        u.state = UpdateState::Available(info("99.0.0"));
        u.dismissed = true;
        assert_eq!(u.skip().as_deref(), Some("99.0.0"));
        assert!(matches!(u.state, UpdateState::Idle));
        assert!(!u.dismissed);
        assert_eq!(u.skip(), None, "nothing to skip any more");

        let mut u = ready(None);
        assert_eq!(u.skip(), None, "a ready update can't be skipped");
    }

    #[test]
    fn a_download_finished_in_the_background_is_announced_again() {
        let mut u = updater();
        u.state = UpdateState::Available(info("99.0.0"));
        u.later();
        u.install_with(|_, _| Ok(None));
        assert!(matches!(wait(&mut u), Some(UpdateEvent::Ready)));
        assert!(!u.dismissed);
    }
}
