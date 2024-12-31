use async_signal::{Signal, Signals};

use crate::errors::AppResult;

/// Table of standard signals
pub static SIGNAL_TABLE: [(&str, Signal); 29] = [
    ("SIGHUP", Signal::Hup),
    ("SIGINT", Signal::Int),
    ("SIGQUIT", Signal::Quit),
    ("SIGILL", Signal::Ill),
    ("SIGTRAP", Signal::Trap),
    ("SIGABRT", Signal::Abort),
    ("SIGBUS", Signal::Bus),
    ("SIGFPE", Signal::Fpe),
    ("SIGKILL", Signal::Kill),
    ("SIGUSR1", Signal::Usr1),
    ("SIGSEGV", Signal::Segv),
    ("SIGUSR2", Signal::Usr2),
    ("SIGPIPE", Signal::Pipe),
    ("SIGALRM", Signal::Alarm),
    ("SIGTERM", Signal::Term),
    ("SIGCHLD", Signal::Child),
    ("SIGCONT", Signal::Cont),
    ("SIGSTOP", Signal::Stop),
    ("SIGTSTP", Signal::Tstp),
    ("SIGTTIN", Signal::Ttin),
    ("SIGTTOU", Signal::Ttou),
    ("SIGURG", Signal::Urg),
    ("SIGXCPU", Signal::Xcpu),
    ("SIGXFSZ", Signal::Xfsz),
    ("SIGVTALRM", Signal::Vtalarm),
    ("SIGPROF", Signal::Prof),
    ("SIGWINCH", Signal::Winch),
    ("SIGIO", Signal::Io),
    ("SIGSYS", Signal::Sys),
];

/// Convert `SIGNAL_TABLE` to a table which is usable in Lua
pub fn signal_table() -> Vec<(&'static str, i32)> {
    SIGNAL_TABLE
        .into_iter()
        .map(|(name, signal)| (name, signal as i32))
        .collect()
}

/// `SIGNAL_TABLE` without signals that cannot be caught
pub fn valid_signals() -> Vec<Signal> {
    let mut signals = Vec::new();
    for (_name, signal) in SIGNAL_TABLE.into_iter() {
        match signal {
            Signal::Kill | Signal::Stop | Signal::Ill | Signal::Fpe | Signal::Segv => continue,
            _ => signals.push(signal),
        }
    }
    signals
}

/// Wait for valid signals
pub async fn signal_wait() -> AppResult<Signals> {
    Ok(Signals::new(valid_signals())?)
}

/// Wrap the C `kill` function
mod libc {
    extern "C" {
        pub fn kill(pid: i32, sig: i32) -> i32;
    }
}

/// Send a signal to a process
#[allow(unsafe_code)]
pub async fn kill(pid: i32, sig: i32) -> AppResult<i32> {
    // SAFETY: safe because an invalid pid or signal will return an error
    let result = unsafe { libc::kill(pid, sig) };
    let error = std::io::Error::last_os_error();
    if result == -1 {
        return Err(error.into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_table() {
        assert_eq!(SIGNAL_TABLE.len(), 29);
    }

    #[test]
    fn test_signal_table_fn() {
        let signals = signal_table();
        assert_eq!(signals.len(), 29);
        assert!(signals.contains(&("SIGKILL", 9)));
    }

    #[test]
    fn test_valid_signals() {
        let signals = valid_signals();
        assert_eq!(signals.len(), 24);
        assert!(!signals.contains(&Signal::Kill));
    }

    #[test]
    fn test_signal_wait() {
        smol::block_on(async {
            assert!(signal_wait().await.is_ok());
        });
    }

    #[test]
    fn test_kill_ok() {
        let pid = std::process::id() as i32;
        smol::block_on(async {
            assert!(kill(pid, 0).await.is_ok());
        });
    }

    #[test]
    fn test_kill_err() {
        let pid = std::process::id() as i32;
        smol::block_on(async {
            assert!(kill(pid, 1337).await.is_err());
        });
    }
}
