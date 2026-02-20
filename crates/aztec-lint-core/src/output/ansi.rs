use std::io::IsTerminal;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Clone, Copy, Debug)]
pub struct Colorizer {
    enabled: bool,
}

impl Colorizer {
    pub fn for_stream(stream: Stream) -> Self {
        Self {
            enabled: color_enabled(stream),
        }
    }

    pub fn error(self, text: &str) -> String {
        self.style(text, "1;31")
    }

    pub fn warning(self, text: &str) -> String {
        self.style(text, "1;33")
    }

    pub fn note(self, text: &str) -> String {
        self.style(text, "1;32")
    }

    pub fn help(self, text: &str) -> String {
        self.style(text, "1;36")
    }

    pub fn accent(self, text: &str) -> String {
        self.style(text, "1;34")
    }

    pub fn style(self, text: &str, code: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

fn color_enabled(stream: Stream) -> bool {
    if std::env::var("CLICOLOR_FORCE")
        .ok()
        .as_deref()
        .is_some_and(|value| value != "0")
    {
        return true;
    }

    if let Ok(value) = std::env::var("CARGO_TERM_COLOR") {
        match value.as_str() {
            "always" => return true,
            "never" => return false,
            _ => {}
        }
    }

    if std::env::var("CLICOLOR")
        .ok()
        .as_deref()
        .is_some_and(|value| value == "0")
    {
        return false;
    }

    match stream {
        Stream::Stdout => std::io::stdout().is_terminal(),
        Stream::Stderr => std::io::stderr().is_terminal(),
    }
}
