#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::{Parser, Subcommand};
use keyring::Entry;
use std::error::Error;
use std::io::ErrorKind;
use std::{
    io::Write,
    mem,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    process::Command,
    thread,
    time::{Duration, Instant},
};
use totp_rs::{Rfc6238, Secret, TOTP};
use zeroize::{Zeroize, Zeroizing};

const DEFAULT_NAME: &str = "[default]";
const DEFAULT_TIMEOUT: &str = "60s";
#[cfg(target_os = "windows")]
const DEFAULT_EXE: &str = "%LocalAppData%\\XIVLauncher\\XIVLauncher.exe";
#[cfg(not(target_os = "windows"))]
const DEFAULT_EXE: &str = "xivlauncher-core";

fn main() -> Result<(), Box<dyn Error>> {
    match Cli::try_parse() {
        Ok(cli) => match cli.command {
            Commands::Save { name, mut secret } => save(&name, &mut secret)?,
            Commands::Delete { name } => delete(&name)?,
            Commands::Send { name, timeout } => send_totp(&name, timeout)?,
            Commands::Launch {
                name,
                timeout,
                path,
            } => {
                launch(&path)?;
                send_totp(&name, timeout)?;
            }
        },
        Err(e) => {
            use clap::error::ErrorKind;
            if let ErrorKind::DisplayHelp | ErrorKind::DisplayVersion = e.kind() {}
            let msg = e.to_string();
            eprintln!("{msg}");
            #[cfg(target_os = "windows")]
            if !std::env::var("XLAUTH_CLI").is_ok() {
                show_message_box(&msg);
            }
            if let ErrorKind::DisplayHelp | ErrorKind::DisplayVersion = e.kind() {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
    };
    Ok(())
}

fn save(name: &str, secret: &mut Vec<String>) -> Result<(), Box<dyn Error>> {
    let validated = validate_secret(secret)?;
    let encoded_secret = Zeroizing::new(validated.to_encoded().to_string());
    let entry = Entry::new("xlauth", name)?;
    entry
        .set_password(encoded_secret.as_str())
        .map_err(|e| format!("TOTP secret was not saved: {}", e))?;
    Ok(())
}

fn validate_secret(secret: &mut Vec<String>) -> Result<Secret, Box<dyn Error>> {
    let joined = {
        let mut j = secret.join("");
        j.retain(|c| !c.is_whitespace());
        Zeroizing::new(j)
    };
    let mut decoded = Zeroizing::new(
        Secret::Encoded(joined.to_string())
            .to_bytes()
            .map_err(|e| format!("TOTP secret is invalid: {}", e))?,
    );
    secret.zeroize();
    Ok(Secret::Raw(mem::take(&mut *decoded)))
}

fn load(name: &str) -> Result<Zeroizing<Vec<u8>>, Box<dyn Error>> {
    let entry = Entry::new("xlauth", name)?;
    let encoded = Zeroizing::new(entry.get_password().map_err(|e| {
        format!(
            "Failed to load TOTP secret \"{}\" from keyring: {}",
            name, e
        )
    })?);
    Ok(Zeroizing::new(
        Secret::Encoded(encoded.to_string())
            .to_bytes()
            .map_err(|e| format!("TOTP secret is invalid: {}", e))?,
    ))
}

fn delete(name: &str) -> Result<(), Box<dyn Error>> {
    let entry = Entry::new("xlauth", name)?;
    entry
        .delete_password()
        .map_err(|e| format!("TOTP secret \"{}\" was not deleted: {}", name, e))?;
    Ok(())
}

fn send_totp(name: &str, timeout: Duration) -> Result<(), Box<dyn Error>> {
    let rfc = Rfc6238::with_defaults(mem::take(&mut *load(name)?))?;
    let totp = TOTP::from_rfc6238(rfc)?;
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 4646));
    let start = Instant::now();

    while start.elapsed() < timeout {
        let remaining = timeout.checked_sub(start.elapsed()).unwrap_or_default();
        match TcpStream::connect_timeout(&addr, remaining) {
            Ok(mut stream) => {
                let totp_code = totp.generate_current()?;
                let pkg_name = env!("CARGO_PKG_NAME");
                let pkg_version = env!("CARGO_PKG_VERSION");
                let request = format!(
                    "GET /ffxivlauncher/{totp_code} HTTP/1.0\r\n\
Host: localhost\r\n\
User-Agent: {pkg_name}/{pkg_version}\r\n\
Content-Length: 0\r\n\
\r\n"
                );
                stream.write_all(request.as_bytes())?;
                return Ok(());
            }
            Err(e) => {
                if e.kind() == ErrorKind::TimedOut {
                    break;
                } else {
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
            }
        }
    }
    Err(format!("connection attempt timed out after {:?}", timeout).into())
}

fn launch(path: &str) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    let resolved_path = expand_str::expand_string_with_env(path)
        .map_err(|e| format!("Failed to expand launcher path '{}': {}", path, e))?;
    #[cfg(not(target_os = "windows"))]
    let resolved_path = path;

    Command::new(resolved_path)
        .spawn()
        .map_err(|e| format!("XIV Launcher failed to start: {}", e))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn show_message_box(msg: &str) {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr::null_mut};
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

    let wide_msg: Vec<u16> = OsStr::new(msg)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MessageBoxW(null_mut(), wide_msg.as_ptr(), wide_msg.as_ptr(), MB_OK);
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Internal flag set by wrapper to force CLI mode
    #[arg(long, hide = true)]
    internal_cli: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Save a TOTP secret
    Save {
        /// Name of the TOTP secret to save
        #[arg(short, long, default_value = DEFAULT_NAME)]
        name: String,

        /// TOTP secret
        #[arg(num_args = 1.., value_delimiter = ' ')]
        secret: Vec<String>,
    },

    /// Delete a TOTP secret
    Delete {
        /// Name of the TOTP secret to delete
        #[arg(short, long, default_value = DEFAULT_NAME)]
        name: String,
    },

    /// Send a TOTP code to XIV Launcher
    Send {
        /// Name of the TOTP secret to use
        #[arg(short, long, default_value = DEFAULT_NAME)]
        name: String,

        /// Timeout duration (e.g. 5s)
        #[arg(short, long, value_parser = humantime::parse_duration, default_value = DEFAULT_TIMEOUT
        )]
        timeout: Duration,
    },

    /// Run XIV Launcher before sending a TOTP code
    Launch {
        /// Name for the TOTP secret to use
        #[arg(short, long, default_value = DEFAULT_NAME)]
        name: String,

        /// Timeout duration (e.g. 5s)
        #[arg(short, long, value_parser = humantime::parse_duration, default_value = DEFAULT_TIMEOUT
        )]
        timeout: Duration,

        /// Path to XIV Launcher
        #[arg(short, long, default_value = DEFAULT_EXE)]
        path: String,
    },
}

/*
MIT License

Copyright © alphaONE2

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
