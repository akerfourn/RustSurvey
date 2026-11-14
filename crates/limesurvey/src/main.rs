use std::{
    error::Error,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use rustsurvey_limesurvey::{
    Limits, LssDocument, MAX_SETTINGS_CONTEXT_BYTES, ResponseTable, SettingsContext, SourceId,
    export_native_with_navigation, native_bindings,
};

const USAGE: &str = concat!(
    "usage: rustsurvey-limesurvey inspect INPUT\n",
    "       rustsurvey-limesurvey export-original INPUT OUTPUT\n",
    "       rustsurvey-limesurvey export-native INPUT_JSON OUTPUT_LSS SURVEY_ID [NAVIGATION_JSON]\n",
    "       rustsurvey-limesurvey export-bindings INPUT_JSON OUTPUT_JSON SURVEY_ID\n",
    "       rustsurvey-limesurvey export-responses INPUT_JSON OUTPUT_LSR\n",
    "       rustsurvey-limesurvey import-responses INPUT_LSR OUTPUT_JSON\n",
    "       rustsurvey-limesurvey export-vv INPUT_JSON ALIASES_JSON OUTPUT_VV\n",
    "       rustsurvey-limesurvey group-settings INPUT_LSS OUTPUT_JSON [CONTEXT_JSON]\n",
    "       rustsurvey-limesurvey set-title INPUT OUTPUT LANGUAGE TITLE"
);

enum Command {
    Inspect(PathBuf),
    Export(PathBuf, PathBuf),
    Native(PathBuf, PathBuf, SourceId, Option<PathBuf>),
    Bindings(PathBuf, PathBuf, SourceId),
    Vv(PathBuf, PathBuf, PathBuf),
    Settings {
        input: PathBuf,
        output: PathBuf,
        context: Option<PathBuf>,
    },
    Responses {
        input: PathBuf,
        output: PathBuf,
        to_xml: bool,
    },
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
        Command::Native(input, output, survey_id, navigation) => {
            let bytes = read_bounded(&input, rustsurvey_domain::MAX_DOCUMENT_BYTES)?;
            let definition = serde_json::from_slice::<rustsurvey_domain::Definition>(&bytes)
                .map_err(|_| io::Error::other("invalid native definition JSON"))?;
            let navigation = match navigation {
                Some(path) => serde_json::from_slice(&read_bounded(&path, 1024)?)
                    .map_err(|_| io::Error::other("invalid navigation settings JSON"))?,
                None => rustsurvey_domain::navigation::NavigationSettings::default(),
            };
            let exported = export_native_with_navigation(
                &definition,
                survey_id,
                navigation,
                Limits::default(),
            )?;
            write_new(&output, &exported.xml)?;
        }
        Command::Bindings(input, output, survey_id) => {
            let bytes = read_bounded(&input, rustsurvey_domain::MAX_DOCUMENT_BYTES)?;
            let definition = serde_json::from_slice::<rustsurvey_domain::Definition>(&bytes)
                .map_err(|_| io::Error::other("invalid native definition JSON"))?;
            let bindings = native_bindings(&definition, survey_id)?;
            write_new(&output, &serde_json::to_vec_pretty(&bindings)?)?;
        }
        Command::Responses {
            input,
            output,
            to_xml,
        } => {
            let limits = Limits::default();
            let bytes = read_bounded(&input, limits.max_bytes)?;
            let output_bytes = if to_xml {
                let table: ResponseTable = serde_json::from_slice(&bytes)
                    .map_err(|_| io::Error::other("invalid response table JSON"))?;
                table.to_xml(limits)?
            } else {
                serde_json::to_vec_pretty(&ResponseTable::parse(&bytes, limits)?)?
            };
            if output_bytes.len() > limits.max_bytes {
                return Err(io::Error::other("output exceeds the configured size limit").into());
            }
            write_new(&output, &output_bytes)?;
        }
        Command::Vv(input, aliases, output) => {
            let limits = Limits::default();
            let table: ResponseTable =
                serde_json::from_slice(&read_bounded(&input, limits.max_bytes)?)
                    .map_err(|_| io::Error::other("invalid response table JSON"))?;
            let aliases = serde_json::from_slice(&read_bounded(&aliases, limits.max_bytes)?)
                .map_err(|_| io::Error::other("invalid response aliases JSON"))?;
            write_new(&output, &table.to_vv(&aliases, limits)?)?;
        }
        Command::Settings {
            input,
            output,
            context,
        } => {
            let document = read_document(&input)?;
            let context: Option<SettingsContext> = context
                .map(|path| {
                    serde_json::from_slice(&read_bounded(&path, MAX_SETTINGS_CONTEXT_BYTES)?)
                        .map_err(|_| io::Error::other("invalid settings context JSON"))
                })
                .transpose()?;
            let report = document
                .group_settings(context.as_ref())
                .map_err(io::Error::other)?;
            write_new(&output, &serde_json::to_vec_pretty(&report)?)?;
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
        [command, input, output] if command == "group-settings" => Ok(Command::Settings {
            input: input.into(),
            output: output.into(),
            context: None,
        }),
        [command, input, output, context] if command == "group-settings" => Ok(Command::Settings {
            input: input.into(),
            output: output.into(),
            context: Some(context.into()),
        }),
        [command, input, aliases, output] if command == "export-vv" => {
            Ok(Command::Vv(input.into(), aliases.into(), output.into()))
        }
        [command] if command == "--help" || command == "-h" => Ok(Command::Help),
        [command, input] if command == "inspect" => Ok(Command::Inspect(input.into())),
        [command, input, output] if command == "export-original" => {
            Ok(Command::Export(input.into(), output.into()))
        }
        [command, input, output, survey_id]
            if command == "export-native" || command == "export-bindings" =>
        {
            let id = survey_id
                .to_str()
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| io::Error::other("survey ID must be a positive integer"))?;
            if command == "export-bindings" {
                return Ok(Command::Bindings(input.into(), output.into(), SourceId(id)));
            }
            Ok(Command::Native(
                input.into(),
                output.into(),
                SourceId(id),
                None,
            ))
        }
        [command, input, output, survey_id, navigation] if command == "export-native" => {
            let id = survey_id
                .to_str()
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| io::Error::other("survey ID must be a positive integer"))?;
            Ok(Command::Native(
                input.into(),
                output.into(),
                SourceId(id),
                Some(navigation.into()),
            ))
        }
        [command, input, output]
            if command == "export-responses" || command == "import-responses" =>
        {
            Ok(Command::Responses {
                input: input.into(),
                output: output.into(),
                to_xml: command == "export-responses",
            })
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
    let input = read_bounded(path, limits.max_bytes)?;
    Ok(LssDocument::parse(&input, limits)?)
}

fn read_bounded(path: &Path, maximum: usize) -> io::Result<Vec<u8>> {
    let mut input = Vec::new();
    File::open(path)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut input)?;
    if input.len() > maximum {
        return Err(io::Error::other("input exceeds the configured size limit"));
    }
    Ok(input)
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
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
