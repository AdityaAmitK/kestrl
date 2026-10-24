//! TCP listening port discovery for a process tree.
use std::process::Command;
pub fn listening(pid: u32) -> Vec<u16> {
    let mut pids = vec![pid];
    let mut i = 0;
    while i < pids.len() {
        if let Ok(o) = Command::new("pgrep")
            .args(["-P", &pids[i].to_string()])
            .output()
        {
            pids.extend(
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter_map(|x| x.parse::<u32>().ok()),
            );
        }
        i += 1;
    }
    let Ok(o) = Command::new("lsof")
        .args([
            "-nP",
            "-a",
            "-iTCP",
            "-sTCP:LISTEN",
            "-p",
            &pids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ])
        .output()
    else {
        return vec![];
    };
    let mut ports = String::from_utf8_lossy(&o.stdout)
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().nth(8))
        .filter_map(|address| address.rsplit(':').next()?.parse::<u16>().ok())
        .collect::<Vec<_>>();
    ports.sort_unstable();
    ports.dedup();
    ports
}
