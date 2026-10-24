//! Spawn a service, tee its child output into the live buffer, and manage its runtime files.
//!
//! This supervisor is the one place in core allowed to write a child's output
//! through to the terminal; core otherwise never prints.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::buffer::Buffer;
use super::ports;
use super::registry;
use super::registry::Service;
use super::server;
use crate::Error;
use crate::Result;
use crate::history::now_ms;

#[cfg(unix)]
fn prepare_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

/// Tee one child stream into the terminal and shared live buffer.
#[expect(
    clippy::print_stdout,
    reason = "the supervisor tees child output to its terminal"
)]
fn forward<R: Read + Send + 'static>(
    stream: super::buffer::Stream,
    reader: R,
    shared: Arc<(Mutex<Buffer>, Condvar)>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            if stream == super::buffer::Stream::Stdout {
                println!("{line}");
            } else {
                eprintln!("{line}");
            }
            if let Ok(mut buffer) = shared.0.lock() {
                buffer.push(stream.clone(), line);
                shared.1.notify_all();
            }
        }
    })
}

fn supervise(name: &str, cmd: &[String]) -> Result<i32> {
    let run_directory = registry::dir()?;
    std::fs::create_dir_all(&run_directory)?;
    let mut command = Command::new(&cmd[0]);
    command
        .args(&cmd[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    prepare_group(&mut command);
    let mut child = command.spawn()?;
    let pid = child.id();
    let interrupted = Arc::new(AtomicBool::new(false));
    let signal_flag = interrupted.clone();
    ctrlc::set_handler(move || signal_flag.store(true, Ordering::SeqCst))
        .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
    let mut service = Service {
        name: name.into(),
        pid,
        command: cmd.join(" "),
        cwd: std::env::current_dir()?.display().to_string(),
        started_at: now_ms(),
        ports: vec![],
    };
    if let Err(error) = registry::write(&service) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Error::Io(error));
    }
    let shared = Arc::new((Mutex::new(Buffer::default()), Condvar::new()));
    if let Err(error) = server::serve(run_directory.join(format!("{name}.sock")), shared.clone()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Error::Io(error));
    }
    let stdout = child
        .stdout
        .take()
        .map(|out| forward(super::buffer::Stream::Stdout, out, shared.clone()));
    let stderr = child
        .stderr
        .take()
        .map(|out| forward(super::buffer::Stream::Stderr, out, shared.clone()));
    let started = std::time::Instant::now();
    let mut next_ports = Duration::ZERO;
    let mut termination_started = None;
    let mut force_killed = false;
    let status = loop {
        if started.elapsed() >= next_ports {
            service.ports = ports::listening(pid);
            let _ = registry::write(&service);
            next_ports = started.elapsed()
                + if started.elapsed() < Duration::from_secs(30) {
                    Duration::from_secs(2)
                } else {
                    Duration::from_secs(10)
                };
        }
        if interrupted.load(Ordering::SeqCst) && termination_started.is_none() {
            let _ = Command::new("kill")
                .args(["-TERM", "--", &format!("-{pid}")])
                .status();
            termination_started = Some(std::time::Instant::now());
        }
        if !force_killed
            && termination_started
                .is_some_and(|started| started.elapsed() >= Duration::from_secs(5))
        {
            let _ = Command::new("kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .status();
            force_killed = true;
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if let Some(thread) = stdout {
        let _ = thread.join();
    }
    if let Some(thread) = stderr {
        let _ = thread.join();
    }
    Ok(status.code().unwrap_or(1))
}

/// Start a service and supervise its process and live logs until it exits.
pub fn run(name: &str, cmd: &[String]) -> Result<i32> {
    if cmd.is_empty() || name.is_empty() || name.contains('/') {
        return Err(Error::InvalidServiceName {
            name: name.to_owned(),
        });
    }
    let run_directory = registry::dir()?;
    std::fs::create_dir_all(&run_directory)?;
    for service in registry::list()? {
        let socket_path = run_directory.join(format!("{}.sock", service.name));
        if service.name == name {
            if UnixStream::connect(&socket_path).is_ok() {
                return Err(Error::ServiceAlreadyRunning {
                    name: name.to_owned(),
                });
            }
            registry::remove(name);
        } else if UnixStream::connect(&socket_path).is_err() {
            registry::remove(&service.name);
        }
    }
    let result = supervise(name, cmd);
    registry::remove(name);
    result
}
