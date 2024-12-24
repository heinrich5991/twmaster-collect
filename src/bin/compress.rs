use clap::App;
use clap::Arg;
use std::error::Error;
use std::fs::File;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;
use twmaster_collect::FileWatcher;
use twmaster_collect::RemoveFileOnDrop;
use twmaster_collect::remove_file_if_present;
use twmaster_collect::remove_file_on_quit;

#[macro_use]
extern crate log;

fn open_if_exists(filename: &str) -> io::Result<Option<File>> {
    match File::open(filename).map(Some) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        res => res,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("Teeworlds Serverlist Compressor")
        .author("heinrich5991 <heinrich5991@gmail.com>")
        .about("Recompress a file every time it changes")
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .takes_value(true)
            .value_name("FILE")
            .default_value("servers.json")
            .help("File to watch")
        )
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .takes_value(true)
            .value_name("OUTPUT")
            .default_value("servers.json.gz")
            .help("Output file")
        )
        .arg(Arg::with_name("compressor")
            .long("compressor")
            .takes_value(true)
            .value_name("COMPRESSOR")
            .default_value("gzip")
            .help("Tool used for compressing the file")
        )
        .get_matches();

    let path = matches.value_of("file").unwrap();
    let compressor = matches.value_of("compressor").unwrap();
    let output = matches.value_of("output").unwrap();
    let temp_output = format!("{}.tmp.{}", output, process::id());

    let watcher = FileWatcher::new(Path::new(&path))?;

    let write_mutex = Arc::new(Mutex::new(false));

    let _remove_on_quit = RemoveFileOnDrop(write_mutex.clone(), output.to_owned());
    {
        let handler_output = output.to_owned();
        let handler_write_mutex = write_mutex.clone();
        ctrlc::set_handler(move || {
            remove_file_on_quit(&handler_write_mutex, &handler_output);
            process::exit(3);
        })?;
    }

    loop {
        if let Some(file) = open_if_exists(path)? {
            debug!("compressing file");
            let status = Command::new(compressor)
                .stdin(file)
                .stdout(File::create(&temp_output)?)
                .status()?;
            if !status.success() {
                return Err(Box::new(io::Error::other(format!("failed to compress (exit status {})", status))));
            }
            let _guard = write_mutex.lock().unwrap_or_else(|e| e.into_inner());
            fs::rename(&temp_output, &output)?;
        } else {
            debug!("file gone, removing output");
            remove_file_if_present(&output)?;
        }

        debug!("waiting for file changes");
        watcher.wait_for_event();
    }
}
