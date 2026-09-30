use super::lexer::SPECS;
use super::*;

fn prose(lang: Lang, src: &str) -> Vec<String> {
    let blocks = extract(lang, src);
    validate(src, &blocks);
    blocks
        .iter()
        .map(|b| b.prose_lines(src).join("\n"))
        .collect()
}

fn validate(src: &str, blocks: &[CommentBlock]) {
    let mut previous = 0;
    for block in blocks {
        assert!(block.range.start >= previous);
        assert!(src.get(block.range.clone()).is_some());
        assert_eq!(
            block.start_line,
            1 + src[..block.range.start]
                .bytes()
                .filter(|&b| b == b'\n')
                .count()
        );
        for line in &block.lines {
            assert!(block.range.start <= line.raw.start && line.raw.end <= block.range.end);
            assert!(line.raw.start <= line.content.start && line.content.end <= line.raw.end);
            assert!(src.get(line.raw.clone()).is_some());
            assert!(src.get(line.content.clone()).is_some());
            assert!(!src[line.raw.clone()].contains('\n'));
            assert!(!src[line.raw.clone()].ends_with('\r'));
        }
        previous = block.range.end;
    }
    for segment in segments(src, blocks) {
        assert_eq!(segment.text.len(), segment.range.len());
    }
}

#[test]
fn merging_kinds_trailing_and_next_code() {
    let src = "// first\n  // second\n\n// third\nx(); // trailing\n// fourth\n/// docs\n//! more docs\n/* block */\n\n  fn next() {}\n// last";
    let blocks = extract(Lang::Rust, src);
    validate(src, &blocks);
    assert_eq!(blocks.len(), 7);
    assert_eq!(blocks[0].prose_lines(src), ["first", "second"]);
    assert_eq!(
        blocks[0].next_code_line.as_deref(),
        Some("x(); // trailing")
    );
    assert!(blocks[2].trailing);
    assert!(!blocks[3].trailing);
    assert_eq!(blocks[4].prose_lines(src), ["docs", "more docs"]);
    assert_eq!(blocks[4].kind, CommentKind::Doc);
    assert_eq!(blocks[5].kind, CommentKind::Block);
    assert_eq!(blocks[5].next_code_line.as_deref(), Some("fn next() {}"));
    assert!(blocks.last().unwrap().next_code_line.is_none());
}

#[test]
fn crlf_unicode_and_block_decoration() {
    let src = "let café = 1; // naïve 🦀\r\n/** Héllo\r\n * 世界\r\n *  indented\r\n */\r\n/// résumé\r\n/// 🦀\r\nfn café() {}\r\n";
    let blocks = extract(Lang::Rust, src);
    validate(src, &blocks);
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].prose_lines(src), ["naïve 🦀"]);
    assert_eq!(&src[blocks[0].lines[0].raw.clone()], "// naïve 🦀");
    assert_eq!(
        blocks[1].prose_lines(src),
        ["Héllo", "世界", " indented", ""]
    );
    assert_eq!(blocks[1].start_line, 2);
    assert_eq!(blocks[2].prose_lines(src), ["résumé", "🦀"]);
    assert_eq!(blocks[2].next_code_line.as_deref(), Some("fn café() {}"));
}

#[test]
fn rust_literals_lifetimes_and_nested_comments() {
    let src = r####"let a = "http://x // no \\\" /* no */";
let b = r###"quoted "# // no /* no */"###;
let c = br##"bytes "# // no"##;
let d = b"// no";
let chars = ('/', '\'', '🦀', '\u{23}', b'#');
fn f<'a>(x: &'a str) { 'label: loop { break 'label; } } // lifetime
/* outer /* nested */ still outer */
/*! inner docs */
/**/ // empty
// final"####;
    let blocks = extract(Lang::Rust, src);
    validate(src, &blocks);
    assert_eq!(blocks.len(), 6);
    assert_eq!(blocks[0].prose_lines(src), ["lifetime"]);
    assert_eq!(
        blocks[1].prose_lines(src),
        ["outer /* nested */ still outer "]
    );
    assert_eq!(blocks[2].kind, CommentKind::Doc);
    assert_eq!(blocks[3].prose_lines(src), [""]);
    assert_eq!(blocks[5].prose_lines(src), ["final"]);
}

#[test]
fn go_raw_strings_and_runes() {
    let src = "s := `raw\n// no /* no */\n`\nr := '\\''\ns = \"http://x // no\" // yes\n// declaration\nfunc f() {}";
    assert_eq!(prose(Lang::Go, src), ["yes", "declaration"]);
    let blocks = extract(Lang::Go, src);
    assert_eq!(blocks[1].kind, CommentKind::Line);
    assert_eq!(blocks[1].next_code_line.as_deref(), Some("func f() {}"));
}

#[test]
fn python_docstring_positions_and_prefixes() {
    let src = r##"#!/usr/bin/python
# encoding comment
r"""Module # prose"""
value = "# string"
assigned = '''not documentation
# still a string
'''
def f(
    x: str = "# no",
):
    # before doc
    """Function prose.
    More prose.
    """
    "not the first statement"
    return f"# no {x}"
class C:
    'Class prose'
    def method(self):
        b"not a docstring"
        "also not a docstring"
async def g():
    u"Async prose"
def one(): "Inline prose"; return 1
if True:
    "not a docstring"
# final"##;
    let blocks = extract(Lang::Python, src);
    validate(src, &blocks);
    let docs: Vec<_> = blocks.iter().filter(|b| b.is_doc()).collect();
    assert_eq!(docs.len(), 5);
    assert!(docs.iter().all(|b| b.kind == CommentKind::Docstring));
    assert_eq!(docs[0].prose_lines(src), ["Module # prose"]);
    assert_eq!(docs[2].prose_lines(src), ["Class prose"]);
    assert_eq!(docs[3].prose_lines(src), ["Async prose"]);
    assert_eq!(docs[4].prose_lines(src), ["Inline prose"]);
    assert_eq!(blocks.last().unwrap().prose_lines(src), ["final"]);
}

#[test]
fn python_strings_are_not_always_docstrings() {
    for src in [
        "'x' + '# string'\n# yes",
        "f'''# no'''\n# yes",
        "b'''# no'''\n# yes",
        "x = 1\n'''# no'''\n# yes",
        "def f():\n    pass\n    '''# no'''\n# yes",
        "if True:\n    '''# no'''\n# yes",
        "x = rf'''# no \\\"'''\n# yes",
    ] {
        assert_eq!(prose(Lang::Python, src), ["yes"], "{src}");
    }
}

#[test]
fn javascript_and_typescript_templates_and_regexes() {
    let src = r##"const s = "http://x // no";
const t = `literal // no ${ {x: `nested ${ "}" /* inner */ } // no`} } /* no */`;
const r = /https?:\/\/[^/*]+/g; // regex
function f() { return /[/*]/; }
f(/[//]/, /a\/\/b/);
const n = 10 / 2 / 5; // division
// final"##;
    for lang in [Lang::JavaScript, Lang::TypeScript] {
        assert_eq!(prose(lang, src), ["inner ", "regex", "division", "final"]);
    }
}

#[test]
fn shell_tokens_quotes_and_heredocs() {
    let src = "#!/bin/sh\necho $# ${x#y} a#b \\# '# no' \"# no\" # yes\ncat <<EOF # opener\n# not a comment\nEOF\ncat <<-'END'\n\t# not a comment\n\tEND\ncat <<A <<\"B\"\n# body a\nA\n# body b\nB\n# final\necho done\n";
    let blocks = extract(Lang::Shell, src);
    validate(src, &blocks);
    assert_eq!(prose(Lang::Shell, src), ["yes", "opener", "final"]);
    assert_eq!(blocks[2].next_code_line.as_deref(), Some("echo done"));
}

#[test]
fn nix_operators_interpolation_and_indented_strings() {
    let src = r##"{ a = "# no /* no */ ${ {x = "# no";}.x /* inside */ }";
b = ''
# no
''${literal} ''' quote ''\n escape
${ "# no" # interpolation
}
'';
c = a // b; # merge operator
/* yes */
}"##;
    assert_eq!(
        prose(Lang::Nix, src),
        ["inside ", "interpolation", "merge operator", "yes "]
    );
}

#[test]
fn elixir_sigils_characters_and_doc_heredocs() {
    let src = r##"x = ?#
y = ?\#
a = ~r/# [#] \/ #/u
b = ~s(a (# no) # no)
c = ~S{# no {nested}}
d = ~w[# no]a
e = ~s|# no|
f = ~s<# no>
g = ~S"""
# no
"""
s = """
# no
"""
@moduledoc """
Module prose.
"""
@doc """Function prose."""
def f(), do: :ok # yes"##;
    let blocks = extract(Lang::Elixir, src);
    validate(src, &blocks);
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].kind, CommentKind::Doc);
    assert_eq!(blocks[0].prose_lines(src), ["", "Module prose.", ""]);
    assert_eq!(blocks[1].prose_lines(src), ["Function prose."]);
    assert_eq!(blocks[2].prose_lines(src), ["yes"]);
}

#[test]
fn zig_multiline_strings_and_no_block_comments() {
    let src = "const x =\n  \\\\ // no /* no */\n  \\\\ more // no\n;\n/* not a comment */\n/// docs\n//! more docs\nconst y = \"http://x\"; // yes";
    assert_eq!(prose(Lang::Zig, src), ["docs\nmore docs", "yes"]);
}

#[test]
fn c_and_cpp_literals() {
    let common =
        "char c = '\\''; const char *s = \"http://x \\\" // no\"; // yes\n/** docs */\nint n;";
    for lang in [Lang::C, Lang::Cpp] {
        assert_eq!(prose(lang, common), ["yes", "docs "]);
    }
    let raw = r##"auto s = R"tag(" // no /* no */ )tag";
auto w = u8R"xy(// no " )xy"; // yes"##;
    assert_eq!(prose(Lang::Cpp, raw), ["yes"]);
}

#[test]
fn ruby_block_comments_require_column_zero() {
    let src = "s = '# no'\ns = \"# no\" # yes\n=begin\nBlock prose\n# literal block text\n=end\n  =begin # ordinary line comment\nx = 1\n# final";
    let blocks = extract(Lang::Ruby, src);
    validate(src, &blocks);
    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[1].kind, CommentKind::Block);
    assert_eq!(
        blocks[1].prose_lines(src),
        ["", "Block prose", "# literal block text", ""]
    );
    assert_eq!(blocks[2].prose_lines(src), ["ordinary line comment"]);
}

#[test]
fn java_csharp_php() {
    let common = "var s = \"http://x // no\"; char c = '/'; // yes\n/** docs */\n/* prose */";
    for lang in [Lang::Java, Lang::CSharp, Lang::Php] {
        assert_eq!(prose(lang, common), ["yes", "docs ", "prose "]);
    }
    assert_eq!(
        prose(Lang::Java, "var s = \"\"\"\n// no\n\"\"\"; // yes"),
        ["yes"]
    );
    assert_eq!(
        prose(
            Lang::CSharp,
            r##"var a = @"quote "" // no"; var b = $@"// no"; // yes"##
        ),
        ["yes"]
    );
    assert_eq!(
        prose(Lang::Php, "<?php\n$x = '# no'; # yes\n// more"),
        ["yes", "more"]
    );
}

#[test]
fn toml_and_yaml_quotes() {
    let common = "a = \"# no\" # yes\nb = '# no'\n# final";
    for lang in [Lang::Toml, Lang::Yaml] {
        assert_eq!(prose(lang, common), ["yes", "final"]);
    }
    let toml = "a = '''\n# no\n'''\nb = \"\"\"\n# no\n\"\"\"\n# yes";
    assert_eq!(prose(Lang::Toml, toml), ["yes"]);
    let yaml = "a: 'it''s # no' # yes\nb: it's plain#text # again\nc: [\"# no\", '# no'] # final";
    assert_eq!(prose(Lang::Yaml, yaml), ["yes", "again", "final"]);
}

#[test]
fn unterminated_literals_comments_and_empty_input() {
    for &(lang, _) in SPECS {
        assert!(extract(lang, "").is_empty());
        assert!(extract(lang, "\n \r\n\t").is_empty());
        validate(
            "\"unterminated 🦀 # //",
            &extract(lang, "\"unterminated 🦀 # //"),
        );
    }
    assert_eq!(prose(Lang::Rust, "/* unfinished 🦀"), ["unfinished 🦀"]);
    assert_eq!(prose(Lang::Rust, "//"), [""]);
    assert_eq!(prose(Lang::Rust, "/**/"), [""]);
    assert_eq!(prose(Lang::Rust, "/*"), [""]);
    assert_eq!(prose(Lang::Rust, "/**"), [""]);
    assert_eq!(prose(Lang::Rust, "/*\n"), ["\n"]);
    assert_eq!(prose(Lang::Ruby, "=begin\nunfinished"), ["\nunfinished"]);
}

#[test]
fn doc_segments_preserve_offsets() {
    let src = "/// café\nfn f() {}\n// 世界";
    let blocks = extract(Lang::Rust, src);
    let segs = segments(src, &blocks);
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0].kind, crate::segment::SegmentKind::DocComment);
    assert_eq!(segs[1].kind, crate::segment::SegmentKind::Comment);
    assert_eq!(segs[0].text, "    café");
    assert_eq!(segs[1].text, "   世界");
    assert_eq!(&src[segs[1].range.clone()], "// 世界");
}

#[test]
fn shell_escaped_token_boundaries_and_here_strings() {
    let src = "echo a\\ #not-comment;# yes\ncat <<< '# no' # here string\n# final";
    assert_eq!(prose(Lang::Shell, src), ["yes", "here string", "final"]);
}

#[test]
fn interpolation_comments_are_code_and_literal_text_is_not() {
    let js = "`text ${\n// real\n{ a: `nested ${1 /* real */}` }\n} // text`; // end";
    assert_eq!(prose(Lang::JavaScript, js), ["real", "real ", "end"]);
    let nix = "let foo' = 1; in \"${\n# real\n{ x = ''# no ${1 /* real */}''; }.x\n} # no\" # end";
    assert_eq!(prose(Lang::Nix, nix), ["real", "real ", "end"]);
}

#[test]
fn ranges_remain_valid_on_arbitrary_unicode_input() {
    let alphabet = [
        'a', 'r', 'b', 'f', '"', '\'', '`', '/', '*', '#', '\\', '\n', '\r', ' ', '\t', '$', '{',
        '}', '(', ')', '[', ']', '?', '~', '<', '>', 'é', '🦀',
    ];
    let mut state = 1234567_u64;
    for _ in 0..500 {
        let mut src = String::new();
        for _ in 0..100 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            src.push(alphabet[(state >> 32) as usize % alphabet.len()]);
        }
        for &(lang, _) in SPECS {
            validate(&src, &extract(lang, &src));
        }
    }
}

#[test]
fn undecorated_block_lines_keep_their_indentation() {
    assert_eq!(
        prose(Lang::C, "/* text\n    code sample\n * prose\n */"),
        ["text\n    code sample\nprose\n"]
    );
    assert_eq!(
        prose(Lang::Python, "\"\"\"First\n    indented\n\"\"\""),
        ["First\n    indented\n"]
    );
}

#[test]
fn shell_arithmetic_shift_is_not_heredoc() {
    let src = "x=$(( 1 << 2 ))\n# one\n(( y <<= 3 ))\n# two\nz=$((1<<2)) # three\ncat <<EOF\n# body\nEOF\n# four\n";
    let blocks = extract(Lang::Shell, src);
    validate(src, &blocks);
    assert_eq!(prose(Lang::Shell, src), ["one", "two", "three", "four"]);
    // `<<2)` is never a heredoc word even outside arithmetic tracking.
    assert_eq!(prose(Lang::Shell, "f <<2)\n# yes"), ["yes"]);
    assert_eq!(prose(Lang::Shell, "cat <<< 2\n# yes"), ["yes"]);
}

#[test]
fn php_attributes_are_not_comments() {
    let src = "<?php\n#[Route('/x', methods: ['GET'])]\nfunction f() {} # yes\n#comment\n";
    assert_eq!(prose(Lang::Php, src), ["yes", "comment"]);
    assert_eq!(
        prose(Lang::Rust, "#[derive(Debug)]\n// yes\nstruct S;"),
        ["yes"]
    );
}

#[test]
fn yaml_block_scalar_bodies_are_not_comments() {
    let src = "a: | # header\n  # not a comment\n\n  line\nb: >-\n  # no\n    # no\nc:\n  - |2\n    # no\n  - x # yes\nd: &anchor |\n  # no\n# final\ne: a > b # plain\n";
    let blocks = extract(Lang::Yaml, src);
    validate(src, &blocks);
    assert_eq!(prose(Lang::Yaml, src), ["header", "yes", "final", "plain"]);
}

#[test]
fn python_nested_docstrings_are_dedented() {
    let src = r#####"import os


@decorator(option={"a": 1})
@other
class Outer(Base, metaclass=Meta):
    """Outer prose.

    Continued outer prose.
        Indented example stays code.
    """

    # a comment before the docstring
    class Inner:
        R"""Raw inner prose."""

        @staticmethod
        def method(
            self,
            mapping: dict[str, int] = {"k": 1, "j": (2, 3)},
            label: str = "x: (y",
            *,
            key=lambda v: v,
        ) -> dict[str, "Outer"]:
            # comment line first
            '''Method prose.

            Second line.
            '''
            value = "not a docstring"
            call("not a docstring")
            return value

    async def run(self) -> None:
        u"""Async prose."""
        await call(
            """not a docstring""",
        )

    def one(self): """One-liner prose."""

    def two(self):
        'Single-quoted prose.'

x = """not a docstring"""
if x:
    def nested():
        """Nested prose."""
"#####;
    let blocks = extract(Lang::Python, src);
    validate(src, &blocks);
    let docs: Vec<Vec<&str>> = blocks
        .iter()
        .filter(|b| b.kind == CommentKind::Docstring)
        .map(|b| b.prose_lines(src))
        .collect();
    assert_eq!(
        docs,
        [
            vec![
                "Outer prose.",
                "",
                "Continued outer prose.",
                "    Indented example stays code.",
                ""
            ],
            vec!["Raw inner prose."],
            vec!["Method prose.", "", "Second line.", ""],
            vec!["Async prose."],
            vec!["One-liner prose."],
            vec!["Single-quoted prose."],
            vec!["Nested prose."],
        ]
    );
    let segs = segments(src, &blocks);
    assert!(segs[0].text.contains("Continued outer prose."));
    assert!(!segs[0].text.contains("Indented example"));
    // Dedented content still maps to the source bytes.
    let method = segs
        .iter()
        .find(|s| s.text.contains("Second line."))
        .unwrap();
    let at = method.text.find("Second line.").unwrap();
    assert_eq!(&src[method.abs(at..at + 12)], "Second line.");
}

#[test]
fn comment_directives_mentions_and_owner_tags_are_blanked() {
    let words = |s: &Segment| s.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let src = "x = 1  # noqa: E501\n\
y = f()  # type: ignore[attr-defined]\n\
# pylint: disable=invalid-name,too-many-args\n\
\n\
# TODO(alice): ask @bob about @param handling, mail sdk@acme.example\n\
z = 2  # noqa: BLE001 - a scheduler must survive its jobs\n\
# fmt: off\n\
# pragma: no cover\n";
    let blocks = extract(Lang::Python, src);
    let text: Vec<String> = segments(src, &blocks).iter().map(words).collect();
    assert_eq!(
        text,
        [
            ": ask about handling, mail sdk",
            "- a scheduler must survive its jobs"
        ]
    );

    // An email address is not a handle; its local part survives the directive pass.
    let found: Vec<_> = directive_ranges("mail sdk@acme.example or @bob")
        .map(|r| r.start)
        .collect();
    assert_eq!(found, [25]);

    let js = "// eslint-disable-next-line no-console, @typescript-eslint/no-explicit-any\n\
foo(); // nolint:errcheck because reasons\n\
// See #[allow(dead_code)] and FIXME(bob) for why.\n\
// We run eslint in CI.\n";
    let blocks = extract(Lang::TypeScript, js);
    let text: Vec<String> = segments(js, &blocks).iter().map(words).collect();
    assert_eq!(
        text,
        ["because reasons", "See and for why. We run eslint in CI."]
    );
}

#[test]
fn code_span_wrapping_onto_next_comment_line_is_blanked() {
    let src = "/// Flags `sitä`, `tätä\n/// ja`, then `pitää` only after the verb.\nfn f() {}\n";
    let blocks = extract(Lang::Rust, src);
    let segs = segments(src, &blocks);
    let text: String = segs.iter().map(|s| s.text.as_str()).collect();
    for word in ["sitä", "tätä", "ja", "pitää"] {
        assert!(!text.contains(word), "{word} should be blanked: {text:?}");
    }
    assert!(text.contains("only after the verb"), "{text:?}");
}
