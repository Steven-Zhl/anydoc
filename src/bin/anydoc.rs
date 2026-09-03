//! The native `anydoc` command: convert one document to Markdown.
//!
//! Mirrors the Node CLI ([`node/cli.js`](https://github.com/firecrawl/anydoc/blob/main/node/cli.js))
//! so the two are interchangeable in scripts, with one exception: `--ocr
//! hosted` needs the Firecrawl API, so this build only accepts the default
//! `--ocr reject` and exits 3 on a document that needs OCR.

use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anydoc::{ConvertError, Format};

const FORMATS: &str = "doc, docx, odt, pdf, ppt, pptx, rtf, epub, xlsx, ods, odp, csv";

const HELP: &str = "anydoc: convert documents to GitHub-Flavored Markdown

Usage:
  anydoc <file> [options]
  anydoc - [options] < file

Converts one document per invocation and writes the Markdown to stdout.
Pass - as the input to read the document from stdin. Never prompts; all
diagnostics go to stderr.

Options:
  -o, --output <path>    Write the Markdown to <path> instead of stdout
  -f, --format <format>  Name the input format instead of detecting it:
                         doc, docx, odt, pdf, ppt, pptx, rtf, epub,
                         xlsx, ods, odp, csv
                         (extension aliases like xls, docm, ppsx resolve
                         to these)
  --assets <dir>         Also write embedded images and objects there,
                         named <stem>-<id>.<ext>
  --ocr <mode>           What to do with a PDF whose pages need OCR:
                         reject (default) exits 3. This build cannot send
                         the document to Firecrawl Parse; the Node CLI
                         (@firecrawl/anydoc) does that with --ocr hosted
  -h, --help             Print this help and exit
  -V, --version          Print the version and exit

The format is detected from the file content; the file extension is the
fallback for signature-less formats (CSV). stdin has no extension, so CSV
input from stdin needs --format csv. Scanned or image-only pages need OCR,
which this build does not do: the document exits 3.

Exit codes:
  0  success
  1  the document could not be read or converted
  2  usage error: unknown option, missing input, or invalid --format
  3  pages of a PDF need OCR

Examples:
  anydoc report.docx
  anydoc slides.pptx -o slides.md
  anydoc - --format csv < data.csv
  curl -s https://example.com/paper.pdf | anydoc -
";

const CONVERSION_ERROR: u8 = 1;
const USAGE_ERROR: u8 = 2;
const NEEDS_OCR: u8 = 3;

fn fail(code: u8, message: &str) -> ! {
    eprintln!("anydoc: {message}");
    std::process::exit(code as i32);
}

struct Args {
    input: Option<String>,
    output: Option<PathBuf>,
    format: Option<Format>,
    assets: Option<PathBuf>,
}

fn parse_args(argv: &[String]) -> Args {
    let mut args = Args { input: None, output: None, format: None, assets: None };
    let mut positional_only = false;
    let mut i = 0;
    while i < argv.len() {
        let mut arg = argv[i].as_str();
        if positional_only || arg == "-" || !arg.starts_with('-') {
            if args.input.replace(arg.to_string()).is_some() {
                fail(
                    USAGE_ERROR,
                    &format!("one document per invocation: unexpected second input '{arg}'"),
                );
            }
            i += 1;
            continue;
        }
        if arg == "--" {
            positional_only = true;
            i += 1;
            continue;
        }
        // Accept --opt value and --opt=value, like the Node CLI.
        let mut inline = None;
        if let Some(eq) = arg.find('=') {
            inline = Some(&arg[eq + 1..]);
            arg = &arg[..eq];
        }
        let mut value = || match inline {
            Some(inline) => inline.to_string(),
            None => {
                i += 1;
                match argv.get(i) {
                    Some(value) => value.clone(),
                    None => fail(USAGE_ERROR, &format!("{arg} requires a value")),
                }
            }
        };
        match arg {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-o" | "--output" => args.output = Some(PathBuf::from(value())),
            "-f" | "--format" => {
                let name = value();
                args.format = Some(match Format::from_extension(&name) {
                    Some(format) => format,
                    None => fail(
                        USAGE_ERROR,
                        &format!("invalid format '{name}'; expected one of: {FORMATS}"),
                    ),
                });
            }
            "--assets" => args.assets = Some(PathBuf::from(value())),
            "--ocr" => {
                let mode = value();
                if mode != "reject" {
                    fail(
                        USAGE_ERROR,
                        &format!(
                            "this build cannot --ocr {mode}; only reject is supported \
                             (the Node CLI, @firecrawl/anydoc, does hosted)"
                        ),
                    );
                }
            }
            _ => fail(USAGE_ERROR, &format!("unknown option '{arg}' (see anydoc --help)")),
        }
        i += 1;
    }
    args
}

fn read_stdin() -> Vec<u8> {
    if std::io::stdin().is_terminal() {
        fail(USAGE_ERROR, "stdin is a terminal; pipe or redirect a document into anydoc -");
    }
    let mut bytes = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
        fail(CONVERSION_ERROR, &format!("failed to read stdin: {e}"));
    }
    bytes
}

fn main() -> ExitCode {
    let args = parse_args(&std::env::args().skip(1).collect::<Vec<_>>());
    let Some(input) = args.input else {
        fail(USAGE_ERROR, "missing input: pass a document path, or - for stdin (see anydoc --help)")
    };

    let (bytes, path): (Vec<u8>, Option<&std::path::Path>) = if input == "-" {
        (read_stdin(), None)
    } else {
        let path = std::path::Path::new(&input);
        match std::fs::read(path) {
            Ok(bytes) => (bytes, Some(path)),
            Err(e) => fail(CONVERSION_ERROR, &format!("{e}")),
        }
    };

    // Without -f the format comes from the file content, with the extension
    // as the fallback; stdin has no extension to fall back to.
    let format = match args
        .format
        .or_else(|| Format::from_bytes(&bytes))
        .or_else(|| path.and_then(Format::from_path))
    {
        Some(format) => format,
        None => fail(
            CONVERSION_ERROR,
            &format!("unsupported input: unrecognized file content and extension: {input}"),
        ),
    };

    let markdown = match anydoc::to_markdown_bytes(&bytes, format) {
        Ok(markdown) => markdown,
        Err(e) => fail(exit_code(&e), &format!("{e}")),
    };

    if let Err(e) = write_output(args.output.as_deref(), &markdown) {
        // Downstream closing the pipe early (e.g. `anydoc big.xlsx | head`)
        // is not a conversion failure.
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            fail(CONVERSION_ERROR, &format!("{e}"));
        }
        std::process::exit(0);
    }

    if let Some(dir) = args.assets {
        if format == Format::Pdf {
            fail(
                CONVERSION_ERROR,
                "PDF conversion produces Markdown directly; --assets is unsupported",
            );
        }
        if let Err(e) = write_assets(&bytes, format, path, &dir) {
            fail(exit_code(&e), &format!("{e}"));
        }
    }

    ExitCode::SUCCESS
}

fn exit_code(error: &ConvertError) -> u8 {
    match error.code() {
        "needsOcr" => NEEDS_OCR,
        _ => CONVERSION_ERROR,
    }
}

fn write_output(output: Option<&std::path::Path>, markdown: &str) -> std::io::Result<()> {
    match output {
        Some(out) => std::fs::write(out, markdown),
        None => std::io::stdout().write_all(markdown.as_bytes()),
    }
}

/// Images and embedded objects live on the document model, not in the
/// Markdown, so they need a second pass to write out.
fn write_assets(
    bytes: &[u8],
    format: Format,
    path: Option<&std::path::Path>,
    dir: &std::path::Path,
) -> Result<(), ConvertError> {
    let document = anydoc::to_document(bytes, format)?;
    std::fs::create_dir_all(dir)?;
    let stem = path
        .and_then(|path| path.file_stem())
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_string());
    for asset in &document.assets {
        let (kind, subtype) = asset.media_type.split_once('/').unwrap_or(("", ""));
        let extension = match kind {
            "image" => subtype.chars().filter(char::is_ascii_alphanumeric).collect(),
            _ => "bin".to_string(),
        };
        std::fs::write(dir.join(format!("{stem}-{}.{extension}", asset.id.0)), &asset.bytes)?;
    }
    eprintln!("wrote {} assets to {}", document.assets.len(), dir.display());
    Ok(())
}
