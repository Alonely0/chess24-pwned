#![allow(clippy::useless_transmute)]
#![cfg_attr(disable_ffmpeg, allow(unused_variables, unused_imports))]
use std::{
    env,
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    panic::catch_unwind,
    path::{Path, PathBuf},
    thread::scope,
};

use crossbeam::channel::{self, Sender};
use intrp::Interpreter;
use rayon::prelude::*;
use video::duration;
#[cfg(not(disable_ffmpeg))] use video::{ffmpeg_join, handle_ffmpeg};

mod board;
mod instr;
mod intrp;
mod video;

fn main() {
    testing();
    // prod();
}

#[allow(dead_code)]
fn testing() {
    let (b, t) = Interpreter::new(
        serde_json::from_reader(BufReader::new(File::open("./0.json").unwrap())).unwrap(),
    )
    .render_frames("./out/");
    #[cfg(not(disable_ffmpeg))]
    ffmpeg_join(
        PathBuf::from("./video.webm"),
        b,
        PathBuf::from("./output.mp4"),
        t,
    );
}

#[allow(dead_code)]
fn prod() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build_global()
        .unwrap();
    let out: PathBuf = env::args().nth(2).unwrap().into();
    let (sx, ref rx) = channel::unbounded();
    scope(move |s| {
        #[cfg(not(disable_ffmpeg))]
        (0..4).for_each(|_| drop(s.spawn(|| handle_ffmpeg(rx))));
        fs::read_dir(env::args().nth(1).unwrap())
            .unwrap()
            .flat_map(|course| {
                let course = course.unwrap();
                let out2 = out.join(course.file_name());
                fs::create_dir(&out2).unwrap_or(());
                fs::read_dir(course.path())
                    .unwrap()
                    .map(move |x| (x.unwrap(), out2.clone()))
            })
            .par_bridge()
            .for_each(|(chapter, out)| {
                let file_name = chapter.file_name();
                let name = file_name.to_string_lossy();
                if let Err(e) = catch_unwind(|| handle_chapter(chapter.path(), &name, &out, &sx)) {
                    BufWriter::new(File::create(out.join(format!("panic_{name}"))).unwrap())
                        .write_all(e.downcast_ref::<&str>().unwrap_or(&"").as_bytes())
                        .unwrap_or(());
                }
            });
        drop(sx)
    });
}

fn handle_chapter(
    mut chapter: PathBuf,
    name: &str,
    out: &Path,
    sx: &Sender<([PathBuf; 3], String, Option<f64>)>,
) {
    dbg!(&name);
    let mut ring_str = format!("tmp_{name}.mp4");
    let p = &ring_str[4..];
    let tmp = &ring_str[..(ring_str.len() - 4)];

    let mut out = out.join(p);
    chapter.push("video.webm");
    if out.exists() && duration(&out).unwrap_or(0.0) >= duration(&chapter).unwrap() - 1.0 {
        dbg!("PREV_DONE");
        out.pop();
        out.push(tmp);
        fs::remove_dir_all(out).unwrap_or(());
        return;
    }
    out.pop();
    chapter.pop();

    chapter.push("0.json");
    let intrp = Interpreter::new(
        match serde_json::from_reader(BufReader::new(File::open(&chapter).unwrap())) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{e}\n{:?}", chapter.as_os_str());
                BufWriter::new(File::create(out.join(format!("broken_{name}"))).unwrap())
                    .write_all(e.to_string().as_bytes())
                    .unwrap_or(());
                return;
            }
        },
    );
    chapter.pop();

    out.push(tmp);
    fs::create_dir_all(&out).unwrap();
    let (v, t) = intrp.render_frames(&out);
    out.pop();

    out.push(p);
    chapter.push("video.webm");
    ring_str.truncate(ring_str.len() - 4);
    sx.send(([chapter, v, out], ring_str, t)).unwrap();
}

fn str(x: &Path) -> &str { x.as_os_str().to_str().unwrap() }
