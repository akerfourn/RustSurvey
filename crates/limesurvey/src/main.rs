use std::{
    error::Error,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use rustsurvey_limesurvey::{Limits, LssDocument};

const USAGE: &str = "usage: rustsurvey-limesurvey inspect INPUT\n       rustsurvey-limesurvey export-original INPUT OUTPUT\n       rustsurvey-limesurvey set-title INPUT OUTPUT LANGUAGE TITLE";

enum Command {
    Inspect(PathBuf),
    Export(PathBuf, PathBuf),
    SetTitle {
        input: PathBuf,
        output: PathBuf,
        language: String,
        title: String,
    },
    Help,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn Error>> {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse_command(&arguments)? {
        Command::Help => println!("{USAGE}"),
        Command::Inspect(input) => {
            let document = read_document(&input)?;
            let mut output = io::BufWriter::new(io::stdout().lock());
            serde_json::to_writer_pretty(&mut output, document.report())?;
            writeln!(output)?;
            output.flush()?;
            if document.report().has_errors() {
                return Ok(ExitCode::from(2));
            }
        }
        Command::Export(input, output) => {
            let document = read_document(&input)?;
            write_new(&output, document.original_xml())?;
        }
        Command::SetTitle {
            input,
            output,
            language,
            title,
        } => {
            let document = read_document(&input)?.with_title(&language, &title)?;
            write_new(&output, document.original_xml())?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn parse_command(arguments: &[OsString]) -> io::Result<Command> {
    match arguments {
        [command] if command == "--help" || command == "-h" => Ok(Command::Help),
        [command, input] if command == "inspect" => Ok(Command::Inspect(input.into())),
        [command, input, output] if command == "export-original" => {
            Ok(Command::Export(input.into(), output.into()))
        }
        [command, input, output, language, title] if command == "set-title" => {
            let language = language
                .to_str()
                .ok_or_else(|| io::Error::other("language must be UTF-8"))?;
            let title = title
                .to_str()
                .ok_or_else(|| io::Error::other("title must be UTF-8"))?;
            Ok(Command::SetTitle {
                input: input.into(),
                output: output.into(),
                language: language.to_owned(),
                title: title.to_owned(),
            })
        }
        _ => Err(io::Error::other(USAGE)),
    }
}

fn read_document(path: &Path) -> Result<LssDocument, Box<dyn Error>> {
    let limits = Limits::default();
    let mut input = Vec::new();
    File::open(path)?
        .take(limits.max_bytes as u64 + 1)
        .read_to_end(&mut input)?;
    Ok(LssDocument::parse(&input, limits)?)
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let result = file.write_all(bytes);
    drop(file);
    if let Err(error) = result {
        fs::remove_file(path).map_err(|_| {
            io::Error::other("write failed and incomplete output could not be removed")
        })?;
        return Err(error);
    }
    Ok(())
}
