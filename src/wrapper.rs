use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::task::JoinHandle;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, Result},
    process::Command,
};

#[tokio::main]
async fn main() -> Result<()> {
    let current_exe = env::current_exe()?;
    let xlauth_path = current_exe
        .parent()
        .map(|p| p.join("xlauth"))
        .unwrap_or_else(|| PathBuf::from("xlauth"));

    let args: Vec<_> = env::args_os().skip(1).collect();

    let mut child = Command::new(xlauth_path)
        .args(&args)
        .env("XLAUTH_CLI", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout_task = forward_stream(child.stdout.take(), Some(tokio::io::stdout()));
    let stderr_task = forward_stream(child.stderr.take(), Some(tokio::io::stderr()));

    let status = child.wait().await?;

    if let Some(h) = stdout_task { let _ = h.await; }
    if let Some(h) = stderr_task { let _ = h.await; }

    std::process::exit(status.code().unwrap_or(1));
}

fn forward_stream<R, W>(reader: Option<R>, writer: Option<W>) -> Option<JoinHandle<()>>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    match (reader, writer) {
        (Some(mut reader), Some(mut writer)) => {
            let handle = tokio::spawn(async move {
                let result: Result<()> = async {
                    tokio::io::copy(&mut reader, &mut writer).await?;
                    writer.flush().await?;
                    Ok(())
                }
                .await;

                if let Err(e) = result {
                    eprintln!("stream forward error: {}", e);
                }
            });
            Some(handle)
        }
        _ => None,
    }
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
