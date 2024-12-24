use clap::App;
use clap::Arg;
use memchr::memchr;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::io;
use std::path::Path;
use twmaster_collect::FileWatcher;

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
        // Instead, flush after every write. However, zstd doesn't forward
        // flushes to the underlying reader, so we need to work around that:
        // https://github.com/gyscos/zstd-rs/issues/170.
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
    let watcher = FileWatcher::new(path)?;

    let stdout = io::stdout();
    let mut stream = DeepFlusher(zstd::Encoder::new(stdout.lock(), 0)?.auto_finish());
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
        watcher.wait_for_event();
    }
}
