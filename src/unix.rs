use async_signal::{Signal, Signals};

use crate::errors::AppResult;

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

pub fn signal_table() -> Vec<(&'static str, i32)> {
    SIGNAL_TABLE
        .into_iter()
        .map(|(name, signal)| (name, signal as i32))
        .collect()
}

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

pub async fn signal_wait() -> AppResult<Signals> {
    Ok(Signals::new(valid_signals())?)
}

mod libc {
    extern "C" {
        pub fn kill(pid: i32, sig: i32) -> i32;
    }
}

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
