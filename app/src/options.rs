//! Command-line options.

/// What `--help` prints.
pub const USAGE: &str = "\
Usage: sofa-tv-browser [OPTIONS] [ADDRESS]

Opens ADDRESS (a web address, or words to search for) or the start screen.

Options:
  --windowed   Start in a window instead of full screen
  --check      Print what this build can play (codecs, DRM) and exit
  --version    Print the version and exit
  -h, --help   Print this help and exit";

/// What the run is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The browser.
    Browse,
    /// `--check`: report and exit.
    Check,
    /// `--help`.
    Help,
    /// `--version`.
    Version,
}

/// The parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// What to do.
    pub mode: Mode,
    /// Start in a window instead of full screen.
    pub windowed: bool,
    /// What to open at start.
    pub start: Option<String>,
}

impl Options {
    /// Reads the arguments that follow the program name.
    ///
    /// # Errors
    ///
    /// An unknown option, or more than one address.
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self { mode: Mode::Browse, windowed: false, start: None };
        let mut only_addresses = false;
        for arg in args {
            if !only_addresses && arg.starts_with('-') {
                match arg.as_str() {
                    "--" => only_addresses = true,
                    "--windowed" => options.windowed = true,
                    "--check" => options.mode = Mode::Check,
                    "--version" => options.mode = Mode::Version,
                    "-h" | "--help" => options.mode = Mode::Help,
                    _ => return Err(format!("unknown option '{arg}'")),
                }
            } else if options.start.is_some() {
                return Err("only one address can be opened at start".to_owned());
            } else {
                options.start = Some(arg);
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn no_arguments_browse_full_screen() {
        assert_eq!(parse(&[]), Ok(Options { mode: Mode::Browse, windowed: false, start: None }));
    }

    #[test]
    fn options_and_an_address_are_read() {
        let options = parse(&["--windowed", "example.com"]).expect("valid arguments");
        assert!(options.windowed);
        assert_eq!(options.start.as_deref(), Some("example.com"));
        assert_eq!(parse(&["--check"]).map(|o| o.mode), Ok(Mode::Check));
        assert_eq!(parse(&["-h"]).map(|o| o.mode), Ok(Mode::Help));
    }

    #[test]
    fn a_dash_address_needs_the_separator() {
        assert!(parse(&["-x"]).is_err());
        assert_eq!(parse(&["--", "-x"]).map(|o| o.start), Ok(Some("-x".to_owned())));
    }

    #[test]
    fn two_addresses_are_refused() {
        assert!(parse(&["a.com", "b.com"]).is_err());
    }
}
