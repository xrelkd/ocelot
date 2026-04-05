use std::{
    collections::HashMap,
    ffi::{CString, OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use nix::{
    sys::{
        signal,
        signal::{SaFlags, SigAction, SigHandler, SigSet, SigmaskHow, Signal},
    },
    unistd,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Command {
    program: PathBuf,
    arguments: Vec<OsString>,
    env: HashMap<OsString, OsString>,
    working_directory: Option<PathBuf>,
    discard_stdout: bool,
    discard_stderr: bool,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            program: PathBuf::from(program.as_ref()),
            arguments: vec![program.as_ref().to_os_string()],
            env: std::env::vars_os().collect(),
            working_directory: None,
            discard_stdout: false,
            discard_stderr: false,
        }
    }

    #[must_use]
    pub fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.arguments.push(arg.as_ref().to_os_string());
        self
    }

    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.arguments.extend(args.into_iter().map(|s| s.as_ref().to_os_string()));
        self
    }

    #[must_use]
    pub fn env<K, V>(mut self, key: K, val: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        drop(self.env.insert(key.as_ref().to_os_string(), val.as_ref().to_os_string()));
        self
    }

    #[must_use]
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env.extend(
            vars.into_iter().map(|(k, v)| (k.as_ref().to_os_string(), v.as_ref().to_os_string())),
        );
        self
    }

    #[must_use]
    pub fn current_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.working_directory = Some(dir.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub const fn discard_stdout(mut self, discard: bool) -> Self {
        self.discard_stdout = discard;
        self
    }

    #[must_use]
    pub const fn discard_stderr(mut self, discard: bool) -> Self {
        self.discard_stderr = discard;
        self
    }

    #[must_use]
    pub const fn is_discard_stdout(&self) -> bool { self.discard_stdout }

    #[must_use]
    pub const fn is_discard_stderr(&self) -> bool { self.discard_stderr }

    /// Execute the command using execve, which replaces the current process
    /// image with the new.
    ///
    /// # Panics
    ///
    /// Panics if the program path cannot be converted to a `CString`.
    #[must_use]
    pub fn exec(&self) -> std::io::Error {
        if let Err(err) = reset_signal_handling() {
            return err;
        }

        // Change to working directory if specified
        if let Some(ref dir) = self.working_directory
            && let Err(err) = unistd::chdir(dir)
        {
            return std::io::Error::from(err);
        }

        let path_c =
            CString::new(self.program.as_os_str().as_bytes()).expect("Invalid program path");

        let args_c = to_cstring_vec(&self.arguments);

        // Convert environment variables to "KEY=VALUE" format
        let env_c: Vec<CString> = self
            .env
            .iter()
            .map(|(k, v)| {
                let mut pair = k.as_bytes().to_vec();
                pair.push(b'=');
                pair.extend_from_slice(v.as_bytes());
                CString::new(pair).expect("Invalid environment variable")
            })
            .collect();

        // Execute execve, which will replace the current process image with the new
        // program. If successful, this function will never return.
        match unistd::execvpe(&path_c, &args_c, &env_c) {
            Ok(_) => unreachable!(
                "The child process has created successfully and should not return from `execvp`"
            ),
            Err(error) => std::io::Error::from(error),
        }
    }
}

fn reset_signal_handling() -> Result<(), std::io::Error> {
    let empty_set = SigSet::empty();
    signal::sigprocmask(SigmaskHow::SIG_SETMASK, Some(&empty_set), None)?;

    let default_handler = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());

    // Reset all signals to default handlers.
    // NOTE: SIGKILL and SIGSTOP cannot be caught or modified, so we skip them.
    // We ignore any errors from sigaction for other signals as they may not be
    // available on all platforms.
    let signals =
        Signal::iterator().filter(|&sig| sig != Signal::SIGSTOP && sig != Signal::SIGKILL);
    for signal in signals {
        #[expect(
            unsafe_code,
            reason = "Calling sigaction to reset signal handlers to default is safe in \
                      single-threaded context after fork"
        )]
        let _ = unsafe { signal::sigaction(signal, &default_handler) };
    }

    Ok(())
}

/// Turns a slice of `OsString` into a Vec of `CString`, suitable for execve
/// arguments.
fn to_cstring_vec(items: &[OsString]) -> Vec<CString> {
    items
        .iter()
        .map(|s| CString::new(s.as_bytes()).expect("Incompatible OsString for CString conversion"))
        .collect()
}
