//! Follow logs of a service Kestrl did not start: a file, a Docker container or a systemd unit,
//! on this machine or over SSH. Followers only read; nothing is started, stopped or restarted.

use crate::{Error, Result};

/// Where an attached service's log lines come from.
#[derive(Clone, Debug)]
pub enum AttachSource {
    /// A log file, followed across rotation.
    File(String),
    /// A Docker container's output.
    Docker(String),
    /// A systemd unit's journal.
    Journal(String),
}

/// What to follow, and on which machine.
#[derive(Clone, Debug)]
pub struct AttachOptions {
    pub source: AttachSource,
    /// SSH destination (`host` or `user@host`, as in `~/.ssh/config`); `None` follows locally.
    pub ssh: Option<String>,
}

/// SSH settings: never prompt, fail fast, and exit when the connection goes quiet for 45 s.
const SSH_OPTIONS: [&str; 8] = [
    "-o",
    "BatchMode=yes",
    "-o",
    "ConnectTimeout=10",
    "-o",
    "ServerAliveInterval=15",
    "-o",
    "ServerAliveCountMax=3",
];

/// Run the follower as service `name` until it exits or is interrupted.
pub fn run(name: &str, options: &AttachOptions) -> Result<i32> {
    super::supervisor::run_attached(name, &command(options)?)
}

/// The command line that follows `options`.
pub fn command(options: &AttachOptions) -> Result<Vec<String>> {
    let follower = follower(&options.source)?;
    let Some(host) = &options.ssh else {
        return Ok(follower);
    };
    if host.is_empty() || host.starts_with('-') {
        return Err(Error::InvalidRequest(format!("invalid SSH host {host:?}")));
    }
    let mut command = vec!["ssh".to_owned()];
    command.extend(SSH_OPTIONS.map(str::to_owned));
    command.extend([host.clone(), "--".to_owned(), remote_script(&follower)]);
    Ok(command)
}

fn follower(source: &AttachSource) -> Result<Vec<String>> {
    let (kind, value) = match source {
        AttachSource::File(path) => ("file", path),
        AttachSource::Docker(container) => ("container", container),
        AttachSource::Journal(unit) => ("unit", unit),
    };
    if value.is_empty() {
        return Err(Error::InvalidRequest(format!("{kind} is empty")));
    }
    let arguments: &[&str] = match source {
        AttachSource::File(path) => &["tail", "-F", "--", path],
        AttachSource::Docker(container) => &["docker", "logs", "-f", "--", container],
        AttachSource::Journal(unit) => &["sh", "-c", JOURNAL_SCRIPT, "kestrl-journal", unit],
    };
    Ok(arguments.iter().map(|&value| value.to_owned()).collect())
}

/// Follow a unit's journal. `journalctl` shows nothing, without an error, when the user may not
/// read the system journal, so say so instead of following in silence. The unit is `$1`.
const JOURNAL_SCRIPT: &str = r#"if [ -z "$(journalctl --unit="$1" --lines=1 --quiet --output=cat 2>/dev/null)" ]; then echo "kestrl: no journal entries are visible for $1; reading the system journal needs membership in systemd-journal or adm, or the unit has not logged yet" >&2; fi; exec journalctl --follow --lines=10 --output=cat --unit="$1""#;

/// A POSIX shell script that runs `follower` and stops it when the SSH session closes. Followers
/// never read stdin, so a reader of the session's stdin sees EOF on disconnect and ends it.
fn remote_script(follower: &[String]) -> String {
    let quoted: Vec<String> = follower.iter().map(|argument| quote(argument)).collect();
    format!(
        "exec 3<&0; {} & p=$!; (cat <&3 >/dev/null; kill $p 2>/dev/null) >/dev/null 2>&1 & wait $p",
        quoted.join(" ")
    )
}

fn quote(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', r"'\''"))
}
