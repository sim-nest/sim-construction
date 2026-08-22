//! Portable Powerproject automation boundary.

use std::path::Path;

use sim_kernel::{CapabilityName, Cx};
use sim_lib_doc_core::{OfficeError, PROCESS_SPAWN_CAPABILITY};

/// Domain-owned seam implemented by a platform automation capsule.
pub trait PowerprojectAutomation: Send + Sync {
    /// Exports the current project to the supplied MSPDI output path.
    fn export_current_project_to_mspdi(&self, out: &Path) -> Result<(), String>;
}

/// Exports the currently open Powerproject project to MSPDI XML through a host bridge.
pub fn export_current_project_to_mspdi(
    cx: &mut Cx,
    automation: &dyn PowerprojectAutomation,
    out: &Path,
) -> Result<(), crate::PowerprojectError> {
    cx.require(&CapabilityName::new(PROCESS_SPAWN_CAPABILITY))
        .map_err(OfficeError::from)?;
    if out.as_os_str().is_empty() {
        return Err(crate::PowerprojectError::OleUnavailable(
            "MSPDI output path is empty".to_owned(),
        ));
    }
    automation
        .export_current_project_to_mspdi(out)
        .map_err(crate::PowerprojectError::OleUnavailable)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use sim_kernel::{DefaultFactory, NoopEvalPolicy};

    use super::*;

    trait GrantOutcome {
        fn expect_granted(self);
    }

    impl GrantOutcome for () {
        fn expect_granted(self) {}
    }

    impl GrantOutcome for sim_kernel::Result<()> {
        fn expect_granted(self) {
            self.unwrap();
        }
    }

    macro_rules! expect_granted {
        ($grant:expr) => {{
            #[allow(clippy::let_unit_value)]
            let grant_result = $grant;
            #[allow(clippy::unit_arg)]
            grant_result.expect_granted();
        }};
    }

    struct Unavailable;
    impl PowerprojectAutomation for Unavailable {
        fn export_current_project_to_mspdi(&self, _out: &Path) -> Result<(), String> {
            Err("automation unavailable".to_owned())
        }
    }

    #[test]
    fn ole_export_is_denied_without_process_spawn_capability() {
        let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));

        let denied = export_current_project_to_mspdi(
            &mut cx,
            &Unavailable,
            &PathBuf::from("/tmp/powerproject.xml"),
        )
        .unwrap_err();

        assert!(matches!(
            denied,
            crate::PowerprojectError::Office(OfficeError::CapabilityDenied(capability))
                if capability.as_str() == PROCESS_SPAWN_CAPABILITY
        ));
    }

    #[test]
    fn unavailable_automation_is_deterministic() {
        let (mut cx, seat) = Cx::new_seated(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
        expect_granted!(seat.grant(&mut cx, CapabilityName::new(PROCESS_SPAWN_CAPABILITY)));

        let unavailable = export_current_project_to_mspdi(
            &mut cx,
            &Unavailable,
            &PathBuf::from("/tmp/powerproject.xml"),
        )
        .unwrap_err();
        assert!(unavailable.to_string().contains("automation unavailable"));
    }
}
