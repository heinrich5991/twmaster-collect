use clap::App;
use clap::Arg;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::mem;
use std::net::Shutdown;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::thread;

#[macro_use]
extern crate log;

struct FileContents {
    update_no: u64,
    contents: Arc<[u8]>,
}

struct FileInfo {
    filename: PathBuf,
    contents: Mutex<FileContents>,
    notifier: Condvar,
}

fn handle_client(
    stream: TcpStream,
    token_mapping: Arc<HashMap<Vec<u8>, FileInfo>>,
) -> Result<(), Box<dyn Error>> {
    debug!("new incoming connection accepted");

    stream.set_nodelay(true)?;

    let mut line = Vec::new();
    let token = BufReader::new(stream).read_until(b'\n', &mut line)?;

    let info = match token_mapping.get(&line) {
        Some(i) => i,
        None => {
            debug!("invalid authentication");
            return Ok(());
        }
    };
    info!("new connection accepted, filename={:?}", info.filename);

    let mut stream = zstd::Encoder::new(stream, 0)?.auto_finish();
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
    /*
    let mut reader = BufReader::new(zstd::Decoder::new(stream)?);
    let mut line = Vec::new();

    line.clear();
    reader.read_until(b'\n', &mut line)?;
    let mut current_update = *info.update_count.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        fs::read(&info.filename)?;
        info.update_count.lock().unwrap_or_else(|e| e.into_inner())
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
*/
}

fn main() -> Result<(), Box<dyn Error>> {
    /*
    env_logger::init();

    let matches = App::new("Teeworlds Serverlist Collector")
        .author("heinrich5991 <heinrich5991@gmail.com>")
        .about("Transmit files without newlines")
        .arg(Arg::with_name("bindaddr")
            .value_name("BINDADDR")
            .required(true)
            .help("Address to listen on")
        )
        .arg(Arg::with_name("token_file")
            .value_name("TOKEN_FILE")
            .required(true)
            .multiple(true)
            .help("List of <TOKEN>:<FILENAME> pairs")
        )
        .get_matches();

    let bindaddr = matches.value_of("bindaddr").unwrap();
    let token_file = matches.values_of("token_file").unwrap();

    let mut seen_filenames = HashSet::new();
    let mut token_mapping = HashMap::new();
    for token_line in csv::Reader::from_path(tokenfile)?.into_deserialize() {
        let token_line: TokenLine = token_line?;
        let protocol_start = format!("twc2 {}\n", token_line.token).into_bytes();
        assert!(token_mapping.insert(protocol_start, token_line.filename.clone()).is_none(), "duplicate token");
        assert!(seen_filenames.insert(token_line.filename), "duplicate filename");
    }
    mem::drop(seen_filenames);
    let token_mapping = Arc::new(token_mapping);

    info!("listening on {}", bindaddr);
    let server = TcpListener::bind(bindaddr)?;
    for stream in server.incoming() {
        let stream = stream?;
        let token_mapping = token_mapping.clone();
        thread::spawn(move || handle_client(stream, token_mapping).unwrap());
    }
*/
    Ok(())
}
