/// Tracks logical statements and suite indentation, without parsing Python.
pub(super) struct Python {
    scopes: Vec<(usize, bool)>,
    first_token: bool,
    header: bool,
    async_prefix: bool,
    depth: usize,
    indent: usize,
    pending_suite: Option<(usize, bool)>,
    inline_first: Option<bool>,
    continued: bool,
}

impl Python {
    pub(super) fn new() -> Self {
        Self {
            scopes: vec![(0, true)],
            first_token: true,
            header: false,
            async_prefix: false,
            depth: 0,
            indent: 0,
            pending_suite: None,
            inline_first: None,
            continued: false,
        }
    }

    pub(super) fn newline(&mut self) {
        if self.depth == 0 && !self.continued {
            self.first_token = true;
            self.header = false;
            self.async_prefix = false;
            self.inline_first = None;
        }
        self.continued = false;
    }

    pub(super) fn token(&mut self, indent: usize, word: &[u8]) -> bool {
        let mut doc = false;
        if self.first_token {
            self.first_token = false;
            if let Some(first) = self.inline_first.take() {
                doc = first;
                self.pending_suite = None;
            } else {
                while self.scopes.len() > 1
                    && self.scopes.last().expect("module scope is never popped").0 > indent
                {
                    self.scopes.pop();
                }
                let suite = self.pending_suite.take();
                if indent > self.scopes.last().expect("module scope is never popped").0 {
                    let first = suite.is_some_and(|(parent, valid)| indent > parent && valid);
                    self.scopes.push((indent, first));
                }
                let scope = self
                    .scopes
                    .last_mut()
                    .expect("module scope is never popped");
                doc = scope.1;
                scope.1 = false;
            }
            self.indent = indent;
            self.header = matches!(word, b"def" | b"class");
            self.async_prefix = word == b"async";
        } else if self.async_prefix {
            self.header = word == b"def";
            self.async_prefix = false;
        }
        doc
    }

    pub(super) fn punctuation(&mut self, byte: u8) {
        match byte {
            b'(' | b'[' | b'{' => self.depth += 1,
            b')' | b']' | b'}' => self.depth = self.depth.saturating_sub(1),
            b':' if self.depth == 0 => {
                self.pending_suite = Some((self.indent, self.header));
                self.inline_first = Some(self.header);
                self.first_token = true;
                self.header = false;
            }
            b';' if self.depth == 0 => {
                self.first_token = true;
                self.inline_first = Some(false);
            }
            b'\\' => self.continued = true,
            _ => {}
        }
    }
}
