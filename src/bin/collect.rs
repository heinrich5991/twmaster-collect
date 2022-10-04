use clap::App;
use clap::Arg;
use std::error::Error;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::process;

#[macro_use]
extern crate log;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("Teeworlds Serverlist Transmitter")
        .author("heinrich5991 <heinrich5991@gmail.com>")
        .about("Receive a file without newlines")
        .arg(Arg::with_name("server")
            .value_name("SERVER")
            .required(true)
            .help("Server to connect to via TCP")
        )
        .arg(Arg::with_name("token")
            .value_name("TOKEN")
            .required(true)
            .help("Token to authenticate against the server")
        )
        .arg(Arg::with_name("file")
            .value_name("FILE")
            .required(true)
            .help("File to write to")
        )
        .get_matches();

    let server = matches.value_of("server").unwrap();
    let token = matches.value_of("token").unwrap();
    let filename = matches.value_of_os("file").unwrap();

    info!("connecting to {}", server);
    let mut stream = TcpStream::connect(server)?;
    info!("connected");
    stream.set_nodelay(true)?;
    stream.write_all(format!("twc2 {}\n", token).as_bytes())?;
    stream.shutdown(Shutdown::Write)?;

    let mut reader = BufReader::new(zstd::Decoder::new(stream)?);
    let mut line = Vec::new();

    let mut temp_filename = filename.to_owned();
    temp_filename.push(&format!(".tmp.{}", process::id()));

    loop {
        line.clear();
        reader.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            // Connection terminated.
            info!("connection closed, filename={:?}", filename);
            return Ok(());
        }
        if line.last().copied() != Some(b'\n') {
            panic!("incomplete write");
        }
        debug!("file received, writing, filename={:?}", filename);
        fs::write(&temp_filename, &line)?;
        fs::rename(&temp_filename, &filename)?;
    }
    /*
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::raw_watcher(tx)?;
    let parent_dir = Path::new(filename).parent().unwrap_or(Path::new(""));
    let parent_dir = if !parent_dir.as_os_str().is_empty() { parent_dir } else { Path::new(".") };
    info!("watching parent directory {:?}", parent_dir);
    watcher.watch(parent_dir, notify::RecursiveMode::NonRecursive)?;

    info!("connecting to {}", server);
    let stream = TcpStream::connect(server)?;
    info!("connected");
    stream.set_nodelay(true)?;
    stream.shutdown(Shutdown::Read)?;
    let mut stream = zstd::Encoder::new(stream, 0)?.auto_finish();
    stream.write_all(format!("twc1 {}\n", token).as_bytes())?;
    stream.flush()?;

    loop {
        let mut contents = fs::read(filename)?;
        // Ensure newline.
        let newline_pos = memchr(b'\n', &contents);
        if let Some(p) = newline_pos {
            if p + 1 != contents.len() {
                panic!("{:?} contains internal newlines at byte {}", filename, p);
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
                notify::RawEvent { path: Some(p), op: Ok(op), .. } if p.file_name() == Some(OsStr::new(filename)) && op.contains(notify::Op::RENAME) => break,
                notify::RawEvent { path: Some(_), op: Ok(_), .. } => continue,
                weird => {
                    warn!("weird event: {:?}", weird);
                    continue;
                },
            }
        }
    }
    */
}
