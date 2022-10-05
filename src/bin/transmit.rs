use clap::App;
use clap::Arg;
use memchr::memchr;
use notify::Watcher;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::io;
use std::path::Path;
use std::sync::mpsc;

#[macro_use]
extern crate log;

struct DeepFlusher<'a, W: Write>(zstd::stream::AutoFinishEncoder<'a, W>);

impl<'a, W: Write> Write for DeepFlusher<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()?;
        // We can't have unbuffered stdout in Rust, feature request is at
        // https://github.com/rust-lang/rust/issues/58326.
        //
        // Instead, flush after every write.
        self.0.get_mut().flush()?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("Teeworlds Serverlist Transmitter")
        .author("heinrich5991 <heinrich5991@gmail.com>")
        .about("Repeatedly send a file without newlines to a remote location")
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .takes_value(true)
            .value_name("FILE")
            .default_value("servers.json")
            .help("File to watch")
        )
        .get_matches();

    let path = Path::new(matches.value_of_os("file").unwrap());

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::raw_watcher(tx)?;
    let parent_dir = path.parent().unwrap_or(Path::new(""));
    let parent_dir = if !parent_dir.as_os_str().is_empty() { parent_dir } else { Path::new(".") };
    let filename = path.file_name().expect("path must have filename");
    info!("watching parent directory {:?}", parent_dir);
    watcher.watch(parent_dir, notify::RecursiveMode::NonRecursive)?;

    let mut stream = DeepFlusher(zstd::Encoder::new(io::stdout().lock(), 0)?.auto_finish());
    stream.write_all(format!("twc2\n").as_bytes())?;
    stream.flush()?;

    loop {
        let mut contents = fs::read(path)?;
        // Ensure newline.
        let newline_pos = memchr(b'\n', &contents);
        if let Some(p) = newline_pos {
            if p + 1 != contents.len() {
                panic!("{:?} contains internal newlines at byte {}", path, p);
            }
        } else {
            contents.push(b'\n');
        }

        debug!("sending file");
        stream.write_all(&contents)?;
        stream.flush()?;

        debug!("waiting for file changes");
        loop {
            match rx.recv().unwrap() {
                notify::RawEvent { path: Some(p), op: Ok(op), .. } if p.file_name() == Some(filename) && op.contains(notify::Op::RENAME) => break,
                notify::RawEvent { path: Some(_), op: Ok(_), .. } => continue,
                weird => {
                    warn!("weird event: {:?}", weird);
                    continue;
                },
            }
        }
    }
}
