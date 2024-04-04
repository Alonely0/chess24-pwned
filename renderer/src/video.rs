use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crossbeam::channel::Receiver;
#[cfg(not(disable_ffmpeg))] pub use ffmpeg::*;

use crate::str;

#[cfg(not(disable_ffmpeg))]
mod ffmpeg {
    use super::*;

    pub fn handle_ffmpeg(rx: &Receiver<([PathBuf; 3], String, Option<f64>)>) {
        while let Ok(([video, concat, mut out], tmp, t)) = rx.recv() {
            ffmpeg_join(&video, &concat, &out, t);
            out.pop();
            out.push(tmp);
            fs::remove_dir_all(&out).unwrap();
        }
    }

    pub fn ffmpeg_join(
        video: impl AsRef<Path>,
        concat: impl AsRef<Path>,
        out: impl AsRef<Path>,
        custom_t: Option<f64>,
    ) -> i32 {
        Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-f", "concat",
            "-safe", "0",
            "-i", str(concat.as_ref()),
            "-i", str(video.as_ref()),
            "-c:a", "copy",
            "-c:v", "h264",
            "-pix_fmt", "yuv420p",
            "-filter_complex", "[1:v]scale=460.8:259.2[top_right];[0:v]scale=588:588[left];color=white:1080x608[bg];[bg][top_right]overlay=W-w-10:10[bg1];[bg1][left]overlay=10:H-h-10",
            "-y",
            "-loglevel", "error",
            "-threads", "0",
            "-t", &custom_t.or_else(|| duration(video.as_ref())).unwrap().to_string(),
            str(out.as_ref())
        ]).status().ok().and_then(|x| x.code()).unwrap_or(-1)
    }
}

pub fn duration(path: impl AsRef<Path>) -> Option<f64> {
    String::from_utf8_lossy(
        &Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                str(path.as_ref()),
            ])
            .output()
            .ok()?
            .stdout,
    )
    .split('\n')
    .next()?
    .parse()
    .ok()
}
