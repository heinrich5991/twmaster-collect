use clap::App;
use clap::Arg;
use std::error::Error;
use std::fs;
use std::io;
use std::io::BufRead as _;
use std::io::BufReader;
use std::process::Command;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;

#[macro_use]
extern crate log;

struct DeleteFileOnDrop(Arc<Mutex<bool>>, String);

impl Drop for DeleteFileOnDrop {
    fn drop(&mut self) {
        delete_file_on_quit(&self.0, &self.1);
    }
}

fn delete_file_on_quit(write_mutex: &Mutex<bool>, file: &str) {
    info!("removing file before quitting");
    {
        let mut _guard = write_mutex.lock().unwrap_or_else(|e| e.into_inner());
        // Already deleted?
        if *_guard {
            return;
        }
        if let Err(e) = remove_file_if_present(file) {
            error!("failed to remove file: {}", e);
        }
        *_guard = true;
    }
}

fn remove_file_if_present(file: &str) -> io::Result<()> {
    let result = fs::remove_file(file);
    if let Err(e) = &result {
        if e.kind() == io::ErrorKind::NotFound {
            return Ok(());
        }
    }
    result
}

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
        .arg(Arg::with_name("delete")
            .long("delete")
            .help("Delete the target file before receiving it for the first time and before quitting")
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
    let delete = matches.is_present("delete");
    let only_updates = matches.is_present("only-updates");
    let command = matches.value_of_os("command").unwrap();
    let args = matches.values_of_os("args").unwrap_or_default();

    let write_mutex = Arc::new(Mutex::new(false));

    let _delete_on_quit;
    if delete {
        debug!("deleting file if present");
        remove_file_if_present(filename)?;

        let handler_filename = filename.to_owned();
        let handler_write_mutex = write_mutex.clone();
        ctrlc::set_handler(move || {
            delete_file_on_quit(&handler_write_mutex, &handler_filename);
            process::exit(3);
        })?;

        _delete_on_quit = DeleteFileOnDrop(write_mutex.clone(), filename.to_owned());
    }

    info!("connecting...");
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
            {
                let _guard = write_mutex.lock().unwrap_or_else(|e| e.into_inner());
                fs::write(&temp_filename, &line)?;
                fs::rename(&temp_filename, &filename)?;
            }
        } else {
            debug!("file received, but ignoring initial state");
        }
        first = false;
    }
}
