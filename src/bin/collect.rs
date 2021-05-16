use clap::App;
use clap::Arg;
use serde::Deserialize;
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
use std::process;
use std::sync::Arc;
use std::thread;

fn handle_client(
    stream: TcpStream,
    token_mapping: Arc<HashMap<Vec<u8>, String>>,
) -> Result<(), Box<dyn Error>> {
    stream.set_nodelay(true)?;
    stream.shutdown(Shutdown::Write)?;
    let mut reader = BufReader::new(zstd::Decoder::new(stream)?);
    let mut line = Vec::new();

    line.clear();
    reader.read_until(b'\n', &mut line)?;
    let filename = match token_mapping.get(&line) {
        Some(f) => f,
        None => return Ok(()),
    };
    let temp_filename = format!("{}.tmp.{}", filename, process::id());

    loop {
        line.clear();
        reader.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            // Connection terminated.
            return Ok(());
        }
        if line.last().copied() != Some(b'\n') {
            panic!("incomplete write");
        }
        fs::write(&temp_filename, &line)?;
        fs::rename(&temp_filename, &filename)?;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("Teeworlds Serverlist Collector")
        .author("heinrich5991 <heinrich5991@gmail.com>")
        .about("Receive files without newlines")
        .arg(Arg::with_name("bindaddr")
            .value_name("BINDADDR")
            .required(true)
            .help("Address to listen on")
        )
        .arg(Arg::with_name("tokenfile")
            .value_name("TOKENFILE")
            .required(true)
            .help("File with list of output-filename:token pairs, one per line")
        )
        .get_matches();

    let bindaddr = matches.value_of("bindaddr").unwrap();
    let tokenfile = matches.value_of_os("tokenfile").unwrap();

    #[derive(Deserialize)]
    struct TokenLine {
        filename: String,
        token: String,
    }
    let mut seen_filenames = HashSet::new();
    let mut token_mapping = HashMap::new();
    for token_line in csv::Reader::from_path(tokenfile)?.into_deserialize() {
        let token_line: TokenLine = token_line?;
        let protocol_start = format!("twc1 {}\n", token_line.token).into_bytes();
        assert!(token_mapping.insert(protocol_start, token_line.filename.clone()).is_none(), "duplicate token");
        assert!(seen_filenames.insert(token_line.filename), "duplicate filename");
    }
    mem::drop(seen_filenames);
    let token_mapping = Arc::new(token_mapping);

    let server = TcpListener::bind(bindaddr)?;
    for stream in server.incoming() {
        let stream = stream?;
        let token_mapping = token_mapping.clone();
        thread::spawn(move || handle_client(stream, token_mapping).unwrap());
    }
    Ok(())
}
