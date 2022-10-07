use clap::App;
use clap::Arg;
use std::error::Error;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::process::Command;
use std::process;

#[macro_use]
extern crate log;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("Teeworlds Serverlist Collector")
        .author("heinrich5991 <heinrich5991@gmail.com>")
        .about("Receive files without newlines")
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .takes_value(true)
            .value_name("FILE")
            .default_value("servers.json")
            .help("File to write to")
        )
        .arg(Arg::with_name("only-updates")
            .long("only-updates")
            .help("Only transmit the file each time it is updated, not when it is just there")
        )
        .arg(Arg::with_name("command")
            .value_name("COMMAND")
            .required(true)
            .help("Command to execute")
        )
        .arg(Arg::with_name("args")
            .value_name("ARG")
            .multiple(true)
            .help("Arguments passed to the command")
        )
        .get_matches();

    let filename = matches.value_of("file").unwrap();
    let only_updates = matches.is_present("only-updates");
    let command = matches.value_of_os("command").unwrap();
    let args = matches.values_of_os("args").unwrap_or_default();

    let mut child = Command::new(command)
        .args(args)
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::piped())
        .spawn()?;
    let child_stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(zstd::Decoder::new(child_stdout)?);

    let mut line = Vec::new();
    line.clear();
    reader.read_until(b'\n', &mut line)?;
    if !line.starts_with(b"twc2") {
        error!("remote program is not a Teeworlds Serverlist Transmitter, exiting…");
        process::exit(2);
    }
    info!("connection established");
    let temp_filename = format!("{}.tmp.{}", filename, process::id());
    let mut first = true;

    loop {
        line.clear();
        reader.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            // Connection terminated.
            info!("connection closed");
            return Ok(());
        }
        if line.last().copied() != Some(b'\n') {
            panic!("incomplete write");
        }
        if !first || !only_updates {
            debug!("file received, writing");
            fs::write(&temp_filename, &line)?;
            fs::rename(&temp_filename, &filename)?;
        } else {
            debug!("file received, but ignoring initial state");
        }
        first = false;
    }
}
