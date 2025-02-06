use regex::Regex;
use std::fs;
use std::io::{self, BufRead, Read};
use std::fs::File;
use std::path::Path;
use std::os::unix::fs::MetadataExt;
use users::get_user_by_uid;

#[derive(Debug)]
#[derive(Default)]
struct Stat {
    pid: u32,
    size: u32,
    rss: u32,
    pss: u32,
    shared_clean: u32,
    shared_dirty: u32,
    private_clean: u32,
    count: u32,
    private_dirty: u32,
    referenced: u32,
    swap: u32
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn pidcmd(pid: u32) -> Option<String>  {
    let filename = format!("/proc/{}/cmdline", pid);
    let mut buffer = Vec::new();
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_e) => return None,
    };

    // return the contents after the raplce step (0x00 -> ' ')
    match file.read_to_end(&mut buffer) {
        Ok(_len) => (),
        _ => return None,
    };
    let content = buffer.iter()
        .map(|&byte| if byte == 0x00 { ' ' } else { byte as char })
        .collect::<String>();
    Some(content)
}

fn piduid(pid: u32) -> std::io::Result<u32> {
    let filename = format!("/proc/{}", pid);
    let metadata = fs::metadata(filename)?;
    let uid = metadata.uid();
    Ok(uid)
}

fn get_username_from_uid(uid: u32) -> Option<String> {
    if let Some(user) = get_user_by_uid(uid) {
        Some(user.name().to_string_lossy().into_owned())
    } else {
        None
    }
}

fn is_kernel(pid: u32) -> bool {
    if let Some(contents) = pidcmd(pid) {
        contents.is_empty()
    } else {
        // treat "/proc/PID/cmdline" found-not-found as kernel mode
        true
    }
}

// search for /proc/xxx/ directory where xxx is all digits
fn pids() -> Vec<u32> {
    let mut vec: Vec<u32> = Vec::new();
    let proc_path = Path::new("/proc");
    if let Ok(dirs) = fs::read_dir(proc_path) {
        for entry in dirs {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    match file_name_str.parse::<u32>() {
                        Ok(value) => {
                            if !is_kernel(value) {
                                vec.push(value);
                            }
                        },
                        _ => (),
                    }
                }
            }
        }
    }
    vec
}

fn show_stat(re: &Regex, pid: u32) {
    let mut stat: Stat = Stat::default();
    stat.pid = pid;

    let mut cmdline = String::new();
    if let Some(cmd) = pidcmd(pid) {
        cmdline = cmd;
        if cmdline.len() > 27 {
            cmdline = cmdline[0..27].to_string();
        }
    }

    let mut username = String::new();
    if let Ok(u) = piduid(pid) {
        match get_username_from_uid(u) {
            Some(uname) => username = uname,
            None => (),
        }
    }
    if username.len() > 8 {
        username = username[0..7].to_string();
    }

    let filename = format!("/proc/{}/smaps", pid);
    if let Ok(lines) = read_lines(filename) {
        for line in lines.flatten() {
            match re.captures(&line) {
                Some(caps) => {
                    let size_label = &caps[1];
                    let size_value = &caps[2];
                    let mut value = 0;
                    if let Ok(val) = size_value.parse::<u32>() {
                        value = val;
                    }
                    match size_label.to_lowercase().as_str() {
                        "size" => stat.size += value,
                        "rss" => stat.rss += value,
                        "pss" => stat.pss += value,
                        "shared_clean" => stat.shared_clean += value,
                        "shared_dirty" => stat.shared_dirty += value,
                        "private_clean" => stat.private_clean += value,
                        "count" => stat.count += value,
                        "private_dirty" => stat.private_dirty += value,
                        "referenced" => stat.referenced += value,
                        "swap" => stat.swap += value,
                        _ => (),
                    }
                },
                None => (),
            }

        }
        println!("{:>5} {:<8} {:<27} {:>8} {:>8} {:>8} {:>8} ", stat.pid, username, cmdline, stat.swap, stat.private_dirty + stat.private_clean, stat.pss, stat.rss);
    }
}

fn main() {
    let vec = pids();
    let re = Regex::new(r"(\w+[_\w]*):\s+(\d+)\s+kB").unwrap();

    println!("  PID User     Command                         Swap      USS      PSS      RSS ");

    for pid in vec {
        show_stat(&re, pid);
    }
}
