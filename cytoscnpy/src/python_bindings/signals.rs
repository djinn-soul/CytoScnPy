use std::sync::{Mutex, OnceLock};

#[cfg(unix)]
type SigHandler = libc::sighandler_t;

#[cfg(unix)]
extern "C" fn sigint_handler(_: libc::c_int) {
    crate::CANCELLED.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(windows)]
unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> i32 {
    if ctrl_type == 0 /* CTRL_C_EVENT */ || ctrl_type == 1
    /* CTRL_BREAK_EVENT */
    {
        crate::CANCELLED.store(true, std::sync::atomic::Ordering::SeqCst);
        1
    } else {
        0
    }
}

#[derive(Default)]
struct SignalGuardState {
    active_guards: usize,
    #[cfg(unix)]
    old_handler: Option<SigHandler>,
    #[cfg(windows)]
    registered: bool,
}

static SIGNAL_GUARD_STATE: OnceLock<Mutex<SignalGuardState>> = OnceLock::new();

fn signal_guard_state() -> &'static Mutex<SignalGuardState> {
    SIGNAL_GUARD_STATE.get_or_init(|| Mutex::new(SignalGuardState::default()))
}

pub(super) struct SignalGuard;

#[allow(unsafe_code)]
impl SignalGuard {
    pub(super) fn new() -> Self {
        let mut state = signal_guard_state()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if state.active_guards == 0 {
            #[cfg(unix)]
            {
                state.old_handler =
                    Some(unsafe { libc::signal(libc::SIGINT, sigint_handler as *const () as _) });
            }

            #[cfg(windows)]
            {
                state.registered = unsafe {
                    windows_sys::Win32::System::Console::SetConsoleCtrlHandler(
                        Some(console_ctrl_handler),
                        1,
                    ) != 0
                };
            }
        }

        state.active_guards += 1;
        Self
    }
}

#[allow(unsafe_code)]
impl Drop for SignalGuard {
    fn drop(&mut self) {
        let mut state = signal_guard_state()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        debug_assert!(state.active_guards > 0);
        state.active_guards = state.active_guards.saturating_sub(1);
        if state.active_guards != 0 {
            return;
        }

        #[cfg(unix)]
        if let Some(old_handler) = state.old_handler.take() {
            unsafe {
                libc::signal(libc::SIGINT, old_handler as _);
            }
        }

        #[cfg(windows)]
        if std::mem::take(&mut state.registered) {
            unsafe {
                windows_sys::Win32::System::Console::SetConsoleCtrlHandler(
                    Some(console_ctrl_handler),
                    0,
                );
            }
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{sigint_handler, SignalGuard};

    extern "C" fn test_sigint_handler(_: libc::c_int) {}

    #[test]
    #[allow(unsafe_code)]
    fn restores_the_original_sigint_handler_after_overlapping_guards() {
        unsafe {
            let original_handler =
                libc::signal(libc::SIGINT, test_sigint_handler as *const () as _);

            let first_guard = SignalGuard::new();
            let second_guard = SignalGuard::new();
            drop(first_guard);

            let active_handler = libc::signal(libc::SIGINT, test_sigint_handler as *const () as _);
            let expected_active_handler = sigint_handler as *const () as usize;
            libc::signal(libc::SIGINT, sigint_handler as *const () as _);

            drop(second_guard);
            let restored_handler =
                libc::signal(libc::SIGINT, test_sigint_handler as *const () as _);
            let expected_restored_handler = test_sigint_handler as *const () as usize;
            libc::signal(libc::SIGINT, original_handler);

            assert_eq!(active_handler as usize, expected_active_handler);
            assert_eq!(restored_handler as usize, expected_restored_handler);
        }
    }
}
