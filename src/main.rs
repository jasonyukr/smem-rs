use users::{get_current_uid, get_user_by_uid};
use sysinfo::{System, SystemExt, ProcessExt};

// pi@buster:~% smem                            
//   PID User     Command                         Swap      USS      PSS      RSS 
//  3621 pi       /usr/bin/dbus-daemon --sess        0      324      692     3492 
//  3623 pi       /usr/lib/gvfs/gvfsd                0     1156     1841     6256 
//  3628 pi       /usr/lib/gvfs/gvfsd-fuse /r        0     1092     2157     6964 
//   596 pi       /lib/systemd/systemd --user        0     1028     2261     7476 
// 10319 pi       /usr/bin/python /usr/bin/sm        0     5588     5666     7148 
//   614 pi       -zsh                               0     9336    10150    12764 
// 10205 pi       -zsh                               0     9548    10377    12988 
//
// [root@kindle root]# smem
//   PID User     Command                         Swap      USS      PSS      RSS 
// 26963 root     sleep 30                           0       76       85      500 
//   884 root     getty -L 115200 /dev/ttymxc        0       80       96      664

fn main() {
    let uid32 = get_current_uid();
    let uidstr = uid32.to_string();

    let mut username = String::new();
    if let Some(user) = get_user_by_uid(uid32) {
        username = format!("{}", &user.name().to_string_lossy());
    }

    let mut system = System::new_all();
    system.refresh_processes();

    for (pid, process) in system.processes() {
        if let Some(user_id) = process.user_id() {
            if uidstr.eq(&user_id.to_string()) {
                println!("UID: {}, USER: {}, PID: {}, Name: {}", uid32, username, pid, process.name());
            }
        }
    }
}

