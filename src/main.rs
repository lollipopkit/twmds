use std::{
    env,
    fs::{self, DirEntry, File},
    io::{BufRead, BufReader, Write},
    process::{self, Stdio},
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use args::Args;
use clap::Parser;
use utils::*;
use wait_timeout::ChildExt;
use rand::seq::SliceRandom;

mod args;
mod macros;
mod utils;

const RATE_LIMIT: &str = "Rate limit exceeded";
const ALREADY_EXISTS: &str = "already exists";
const LOGGED_IN: &str = "Logged in";
//const ACCESS_CTRL: &str = "Authorization: Denied by access control: Missing LdapGroup(visibility-admins); Missing LdapGroup(visibility-custom-suspension)";
const DOWNLOADED: &str = "Downloaded";
const ERROR: &str = "error";
const DAY_SECS: u64 = 60 * 60 * 24;

fn main() -> Result<()> {
    let args = Args::parse();

    let pwd = env::var("PWD").unwrap_or_else(|_| ".".to_string());

    let mut folders: Vec<DirEntry> = fs::read_dir(&pwd)?
        .filter_map(|entry| entry.ok())
        .collect();
    let mut rng = rand::thread_rng();
    folders.shuffle(&mut rng);

    let total_count = folders.len();
    println!("🚀 开始，总计 {} 个", total_count);

    let mut counter = 0;
    let mut perm_skip_names = Vec::new();
    for entry in folders {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                println!("😣 获取文件类型失败：{}", err);
                continue;
            }
        };
        if file_type.is_file() {
            continue;
        }
        let user_name = if let Ok(name) = entry.file_name().into_string() {
            name
        } else {
            println!("😣 未能获取文件名");
            continue;
        };
        if args.skip_dirs.contains(&user_name) {
            println!("\n👋🏻 自定义跳过 {}", &user_name);
            continue;
        }

        counter += 1;
        println!("\n[{}/{}] {}", counter, total_count, &user_name);

        let img_dir = format!("{}/{}", &user_name, "img");
        let vid_dir = format!("{}/{}", &user_name, "video");
        if create_dir_ignore_exists(&img_dir).is_err() {
            continue;
        }
        if create_dir_ignore_exists(&vid_dir).is_err() {
            continue;
        }

        let perm_skip_flag = format!("{}/{}", &user_name, ".perm_skip");
        if let Ok(meta) = fs::metadata(&perm_skip_flag) {
            if meta.is_file() {
                println!("💡 永久跳过");
                continue;
            }
        }

        let skip_path = format!("{}/{}", &user_name, ".skip");
        if !args.ignore_skip_file {
            let skip_file_stat = fs::metadata(&skip_path);
            match skip_file_stat {
                Ok(meta) => {
                    let mod_ts = if let Ok(mod_ts) = meta.modified() {
                        mod_ts
                    } else {
                        println!("😣 未能获取文件修改时间");
                        continue;
                    };
                    let mod_ts = mod_ts.elapsed().unwrap_or_default().as_secs();
                    if mod_ts < DAY_SECS {
                        println!("🕒 上次更新在 {} 前，跳过", secs_to_human(mod_ts));
                        continue;
                    }
                }
                _ => (),
            }
        }

        let retweet_only_path = format!("{}/{}", &user_name, ".retweet_only");
        let mut only_retweet = false;
        if let Ok(meta) = fs::metadata(retweet_only_path) {
            if meta.is_file() {
                only_retweet = true;
            }
        }

        let mut need_login = !args.no_login;
        let login_path = format!("{}/{}", &user_name, ".login");
        if let Ok(meta) = fs::metadata(&login_path) {
            if meta.is_file() {
                need_login = true;
            }
        }

        let mut cmd = process::Command::new("twmd");
        cmd.arg("-B");
        if need_login {
            cmd.arg("--login");
        }
        cmd.arg("--all");
        cmd.arg("--update");
        if only_retweet {
            cmd.arg("--retweet-only");
        }
        cmd.arg("--user");
        cmd.arg(&user_name);
        cmd.stdout(Stdio::piped());
        let mut child = if let Ok(child) = cmd.spawn() {
            child
        } else {
            println!("🚫 未能启动 twmd");
            continue;
        };

        if let Some(stdout) = child.stdout.take() {
            // 总行数
            let mut total_lines = match args.no_login {
                true => 0,
                false => -1,
            };
            let mut exists_lines = 0;
            let mut err_lines = 0;
            let mut dl_lines = 0;
            // 不正常的行（非 already exists、Download、error）
            let mut abnormal_lines = Vec::new();

            // 按行读取输出
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                total_lines += 1;
                let line = if let Ok(line) = line {
                    line.trim().to_string()
                } else {
                    continue;
                };

                print_flush!("{}{}", "\x1b[0G\x1b[2K", line);

                if line.contains(format!("User '{}' not found", &user_name).as_str()) {
                    perm_skip_names.push(user_name.clone());
                    match File::create(&perm_skip_flag) {
                        Ok(_) => println!("\n🚫 用户不存在，已生成 {}", &perm_skip_flag),
                        Err(err) => println!("\n🚫 生成永久跳过失败：{}", err),
                    }
                    if child.kill().is_ok() {
                        println!("🚫 已终止");
                    }
                    break;
                }
                if line == LOGGED_IN {
                    continue;
                }
                if line.contains(RATE_LIMIT) {
                    println!("\n🚫 {}", RATE_LIMIT);
                    if child.kill().is_ok() {
                        println!("🚫 已终止");
                    }
                    sleep(Duration::from_secs(args.sleep * 30));
                    break;
                }
                if line.ends_with(ALREADY_EXISTS) {
                    exists_lines += 1;
                    continue;
                }
                if line == ERROR {
                    err_lines += 1;
                    continue;
                }
                if line.starts_with(DOWNLOADED) {
                    dl_lines += 1;
                    continue;
                }
                abnormal_lines.push(line);
            }

            if total_lines == 0 {
                File::create(format!("{}/{}", &user_name, ".login"))?;
                println!("🚫 无输出，已生成 .login");
                continue;
            }

            println!(
                "\n📦 总 {}，下载 {}，存在 {}，失败 {}",
                total_lines, dl_lines, exists_lines, err_lines
            );

            if err_lines <= total_lines / 10 {
                File::create(&skip_path)?;
                println!("💡 生成 .skip 文件");
            } else if err_lines >= total_lines / 3 {
                File::create(&login_path)?;
                println!("🚫 失败率过高，已生成 {}", &login_path);
            }

            if !abnormal_lines.is_empty() {
                let path = format!("{}/{}", &user_name, ".abnormal");
                fs::write(&path, abnormal_lines.join("\n"))?;
                println!("🚫 异常行已写入 {}", &path);
            }
        }

        match child.wait_timeout(Duration::from_secs(60 * 5)) {
            Ok(Some(_)) => (),
            Ok(None) => {
                println!("🚫 超时");
                if child.kill().is_ok() {
                    println!("🚫 已终止");
                }
            }
            Err(err) => {
                println!("🚫 等待 twmd 失败：{}", err);
            }
        }
        sleep(Duration::from_secs(args.sleep));
    }

    if !perm_skip_names.is_empty() {
        println!("🚫 永久跳过：{:?}", perm_skip_names);
    }
    println!("🎉 完成");

    Ok(())
}
