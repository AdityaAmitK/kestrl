//! Spawn a service, tee its child output into the live buffer, and manage its runtime files.
//!
//! This supervisor is the one place in core allowed to write a child's output
//! through to the terminal; core otherwise never prints.

use super::{
    buffer::Buffer,
    ports,
    registry::{self, Service},
    server,
};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
fn prepare_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

#[allow(clippy::print_stdout)]
fn forward<R: Read + Send + 'static>(
    stream: &'static str,
    reader: R,
    shared: Arc<(Mutex<Buffer>, Condvar)>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            if stream == "stdout" {
                println!("{line}");
            } else {
                eprintln!("{line}");
            }
            if let Ok(mut buffer) = shared.0.lock() {
                buffer.push(stream, line);
                shared.1.notify_all();
            }
        }
    })
}

fn supervise(name: &str, cmd: &[String]) -> std::io::Result<i32> {
    let d = registry::dir()?;
    std::fs::create_dir_all(&d)?;
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
        .map_err(|error| std::io::Error::other(error.to_string()))?;
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
        return Err(error);
    }
    let shared = Arc::new((Mutex::new(Buffer::default()), Condvar::new()));
    if let Err(error) = server::serve(d.join(format!("{name}.sock")), shared.clone()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let stdout = child
        .stdout
        .take()
        .map(|out| forward("stdout", out, shared.clone()));
    let stderr = child
        .stderr
        .take()
        .map(|out| forward("stderr", out, shared.clone()));
    let started = std::time::Instant::now();
    let mut next_ports = Duration::ZERO;
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
        if interrupted.load(Ordering::SeqCst) {
            let _ = Command::new("kill")
                .args(["-TERM", "-", &pid.to_string()])
                .status();
            let _ = child.kill();
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

pub fn run(name: &str, cmd: &[String]) -> std::io::Result<i32> {
    if cmd.is_empty() || name.is_empty() || name.contains('/') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "service name and command are required",
        ));
    }
    let d = registry::dir()?;
    std::fs::create_dir_all(&d)?;
    if let Ok(services) = registry::list() {
        for service in services {
            let sock = d.join(format!("{}.sock", service.name));
            if alive(service.pid) {
                if service.name == name {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "service already running",
                    ));
                }
            } else {
                registry::remove(&service.name);
            }
            if service.name == name && sock.exists() {
                registry::remove(name);
            }
        }
    }
    let result = supervise(name, cmd);
    registry::remove(name);
    result
}
