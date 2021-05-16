use clap::App;
use clap::Arg;
use memchr::memchr;
use notify::Watcher;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::sync::mpsc;

fn main() -> Result<(), Box<dyn Error>> {
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
        .get_matches();

    let filename = matches.value_of_os("file").unwrap();
    let server = matches.value_of("server").unwrap();
    let token = matches.value_of("token").unwrap();

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::raw_watcher(tx)?;
    watcher.watch(".", notify::RecursiveMode::NonRecursive)?;

    let stream = TcpStream::connect(server)?;
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

        stream.write_all(&contents)?;
        stream.flush()?;

        loop {
            match rx.recv().unwrap() {
                notify::RawEvent { path: Some(p), op: Ok(op), .. } if p.file_name() == Some(OsStr::new(filename)) && op.contains(notify::Op::RENAME) => break,
                notify::RawEvent { path: Some(_), op: Ok(_), .. } => continue,
                weird => {
                    println!("weird event: {:?}", weird);
                    continue;
                },
            }
        }
    }
}
