use std::fs;
use std::io::{self, BufRead, Read, Result};
use std::fs::File;
use std::path::Path;
use std::os::unix::fs::MetadataExt;
use users::get_user_by_uid;
use std::collections::HashMap;
use std::io::Write;
use std::process;
use std::env;

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

fn pidcmd(pid: u32) -> Result<String> {
    let filename = format!("/proc/{}/cmdline", pid);
    let mut file = File::open(filename)?;

    // return the contents after the raplce step (0x00 -> ' ')
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let content = buffer.iter()
        .map(|&byte| if byte == 0x00 { ' ' } else { byte as char })
        .collect::<String>();
    Ok(content)
}

fn piduid(pid: u32) -> Result<u32> {
    let filename = format!("/proc/{}", pid);
    let metadata = fs::metadata(filename)?;
    let uid = metadata.uid();
    Ok(uid)
}

fn username(uid: u32) -> Option<String> {
    if let Some(user) = get_user_by_uid(uid) {
        Some(user.name().to_string_lossy().into_owned())
    } else {
        None
    }
}

fn is_kernel(pid: u32) -> bool {
    if let Ok(contents) = pidcmd(pid) {
        contents.is_empty()
    } else {
        true // treat "/proc/PID/cmdline" file-not-found case as kernel mode
    }
}

// search for /proc/PID/ directory where PID part all digits
fn pids() -> Result<Vec<u32>> {
    let mut vec: Vec<u32> = Vec::new();
    let proc_path = Path::new("/proc");
    let dirs = fs::read_dir(proc_path)?;
    for entry in dirs {
        let entry = entry?;
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
                    Err(_) => (),
                }
            }
        }
    }
    Ok(vec)
}

fn get_kb(line: &str) -> u32 {
    let vec: Vec<&str> = line.split_whitespace().collect();
    if vec.len() == 2 {
        if let Ok(val) = vec[0].parse::<u32>() {
            return val;
        }
    }
    0
}

fn show_stat(ucache: &mut HashMap<u32, String>, pid: u32) {
    let mut stdout = io::stdout();
    let mut stat: Stat = Stat::default();
    stat.pid = pid;

    let filename = format!("/proc/{}/smaps", pid);
    if let Ok(lines) = read_lines(filename) {
        for mut line in lines.flatten() {
            line = line.to_lowercase();

            if let Some(kb_str) = line.strip_prefix("size:") {
                stat.size += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("rss:") {
                stat.rss += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("pss:") {
                stat.pss += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("shared_clean:") {
                stat.shared_clean += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("shared_dirty:") {
                stat.shared_dirty += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("private_clean:") {
                stat.private_clean += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("count:") {
                stat.count += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("private_dirty:") {
                stat.private_dirty += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("referenced:") {
                stat.referenced += get_kb(kb_str);
            } else if let Some(kb_str) = line.strip_prefix("swap:") {
                stat.swap += get_kb(kb_str);
            }
        }
        let mut cmdline = String::new();
        if let Ok(cmd) = pidcmd(pid) {
            cmdline = cmd;
            if cmdline.len() > 27 {
                cmdline = cmdline[0..27].to_string();
            }
        }

        let mut uname = String::new();
        if let Ok(u) = piduid(pid) {
            if let Some(found) = ucache.get(&u) {
                uname = found.to_string();
            } else {
                match username(u) {
                    Some(un) => {
                        ucache.insert(u, un.clone());
                        uname = un;
                    },
                    None => (),
                };
            }
        }

        let res = writeln!(&mut stdout, "{:>5} {:<8} {:<27} {:>8} {:>8} {:>8} {:>8} ",
            stat.pid, uname, cmdline, stat.swap, stat.private_dirty + stat.private_clean, stat.pss, stat.rss);
        // Terminate the app if the writeln!() has failed due to the error like PIPE-ERROR
        match res {
            Ok(_) => (),
            Err(_e) => { process::exit(1) },
        }
    }
}

fn print_header() {
    println!("  PID User     Command                         Swap      USS      PSS      RSS ");
}

fn main() {
    let mut ucache: HashMap<u32, String> = HashMap::new();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].parse::<u32>() {
            Ok(n) => {
                print_header();
                show_stat(&mut ucache, n);
                return ();
            },
            Err(_e) => (),
        }
    }

    if let Ok(vec) = pids() {
        print_header();
        for pid in vec {
            show_stat(&mut ucache, pid);
        }
    }
}
