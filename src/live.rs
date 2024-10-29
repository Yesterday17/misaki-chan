use crate::bot::BOT;
use crate::config::CONFIG;
use crate::parser::get_og_title;
use chrono::Utc;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use teloxide::prelude::*;
use tokio::process::Child;
use tokio::sync::Mutex;

pub(crate) static LIVES: Lazy<Mutex<HashMap<i64, LiveBody>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct StreamArgs {
    /// 直播 ID
    pub room_index: i64,
    /// 拉流参数
    pub args: Vec<String>,
    /// 推流码
    pub live_key: String,
    /// 是否使用 srt
    pub srt: bool,
}

impl StreamArgs {
    pub async fn build(&self, url: &str, user: ChatId) -> anyhow::Result<LiveBody> {
        let title = get_og_title(url)
            .await
            .unwrap_or_default()
            .unwrap_or(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
            .replace("/", "／");
        let record = CONFIG
            .record_root
            .as_deref()
            .and_then(|path| PathBuf::from_str(&path).ok())
            .map(|path| path.join(&*format!("{{plugin}}/{title}/{{time}}.ts")));

        let mut process_pipe_source = tokio::process::Command::new(CONFIG.path.streamlink())
            .args(&self.args)
            .arg(url)
            .arg("best")
            .args(&match &record {
                Some(path) => {
                    let path = path.to_str().unwrap_or_default();
                    vec!["--record-and-pipe", path]
                }
                None => vec!["-O"],
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let address = if self.srt {
            let key = self.live_key.replace("&", ",").replace("?", ",");
            format!(
                "srt://6721.livepush.myqcloud.com:9000?streamid=#!::h=6721.livepush.myqcloud.com,r=live/{key}"
            )
        } else {
            format!(
                "rtmp://qqgroup.6721.livepush.ilive.qq.com/trtc_1400526639/{}",
                self.live_key
            )
        };

        let format = if self.srt { "mpegts" } else { "flv" };
        #[rustfmt::skip]
        let mut process_ffmpeg = tokio::process::Command::new(CONFIG.path.ffmpeg())
            .args(&[
                "-re",                          // realtime mode
                "-i", "pipe:0",                 // read from stdin
                "-c:v", "copy",                 // copy video frames
                "-c:a", "aac",                  // re-encode audio to:
                "-ar", "44100",                 //   - 44100 (maybe 48000 later?)
                "-b:a", "192k",                 //   - 192k
                "-f", format,                   // format: flv / mpegts
                // error recovery
                "-drop_pkts_on_overflow", "1",  // drop packets for realtime playback
                "-attempt_recovery", "1",       // enable error recovery
                "-max_recovery_attempts", "5",  // retry 5 times
                "-recovery_wait_time", "1",     // wait for 1 second before recovery
            ])
            .arg(address)
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        // stdio pipe
        let mut stdout = process_pipe_source.stdout.take().unwrap();
        let mut stdin = process_ffmpeg.stdin.take().unwrap();
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut stdout, &mut stdin).await;
            // send end to user
            let _ = BOT
                .get()
                .unwrap()
                .send_message(user, format!("推流已结束。\n\n录制标题为：{title}"))
                .await;
        });

        Ok(LiveBody {
            user,
            source: process_pipe_source,
            ffmpeg: process_ffmpeg,
        })
    }
}

pub struct LiveBody {
    /// 接收直播通知的用户
    pub user: ChatId,
    /// pipe source 进程
    pub source: Child,
    /// ffmpeg 进程
    pub ffmpeg: Child,
}
