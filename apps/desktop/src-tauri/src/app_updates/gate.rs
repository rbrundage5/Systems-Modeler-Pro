//! Startup-only update ownership. Once modeling starts, this gate never reopens.
#[derive(Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Checking,
    Installing,
    Open,
}

pub struct Gate<T> {
    pub phase: Phase,
    pending: Option<T>,
}

impl<T> Default for Gate<T> {
    fn default() -> Self {
        Self {
            phase: Phase::Ready,
            pending: None,
        }
    }
}

impl<T> Gate<T> {
    pub fn begin_check(&mut self) -> Result<(), &'static str> {
        if self.phase != Phase::Ready {
            return Err("An update operation is already active, or the application is open.");
        }
        self.pending = None;
        self.phase = Phase::Checking;
        Ok(())
    }

    pub fn finish_check(&mut self, pending: Option<T>) -> bool {
        if self.phase != Phase::Checking {
            return false;
        }
        self.pending = pending;
        self.phase = Phase::Ready;
        true
    }

    pub fn begin_install(&mut self) -> Result<T, &'static str> {
        if self.phase != Phase::Ready {
            return Err("Updates can only be installed before opening the application.");
        }
        let update = self
            .pending
            .take()
            .ok_or("Check for an available update first.")?;
        self.phase = Phase::Installing;
        Ok(update)
    }

    pub fn install_failed(&mut self) {
        if self.phase == Phase::Installing {
            self.phase = Phase::Ready;
        }
    }

    pub fn open(&mut self) -> Result<(), &'static str> {
        if self.phase == Phase::Installing {
            return Err("Wait for installation to finish before opening the application.");
        }
        self.pending = None;
        self.phase = Phase::Open;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_check_cannot_reopen_modeling_session() {
        let mut gate = Gate::default();
        gate.begin_check().unwrap();
        gate.open().unwrap();
        assert!(!gate.finish_check(Some("update")));
        assert!(gate.begin_install().is_err());
        assert!(gate.begin_check().is_err());
        assert_eq!(gate.phase, Phase::Open);
    }

    #[test]
    fn installation_is_exclusive_and_requires_a_checked_update() {
        let mut gate = Gate::default();
        assert!(gate.begin_install().is_err());
        gate.begin_check().unwrap();
        assert!(gate.begin_check().is_err());
        gate.finish_check(Some("verified metadata"));
        assert_eq!(gate.begin_install().unwrap(), "verified metadata");
        assert!(gate.open().is_err());
        assert!(gate.begin_install().is_err());
        assert!(gate.begin_check().is_err());
    }

    #[test]
    fn download_or_signature_failure_allows_opening_but_not_reusing_update() {
        let mut gate = Gate::default();
        gate.begin_check().unwrap();
        gate.finish_check(Some("update"));
        gate.begin_install().unwrap();
        gate.install_failed();
        assert!(gate.begin_install().is_err());
        gate.open().unwrap();
        assert_eq!(gate.phase, Phase::Open);
    }

    #[test]
    fn no_update_or_check_failure_allows_retry_or_open() {
        let mut gate = Gate::<()>::default();
        gate.begin_check().unwrap();
        gate.finish_check(None);
        assert!(gate.begin_install().is_err());
        gate.begin_check().unwrap();
        gate.finish_check(None);
        gate.open().unwrap();
    }
}
