use std::io::{self, Write};
use std::io::BufRead;

use regex::Regex;
use std::cmp::max;
use std::fmt::*;
use crate::pipem::Fragment::{StaticValue, SingleField, FieldRange, UnboundedFieldRange};

pub const SPACE_BYTE: u8 = b" "[0];
pub const COMMA_BYTE: u8 = b","[0];
pub const NEWLINE_BYTE: u8 = b"\n"[0];
pub const PIPE_BYTE: u8 = b"|"[0];

#[derive(PartialEq, Debug)]
struct FieldValues<'a> {
    raw_record: &'a [u8],
    raw_len: usize,
    delimiter_indexes: Vec<usize>,
}

impl FieldValues<'_> {
    fn parse(raw_record: &[u8], field_separator: u8) -> FieldValues {
        let mut offset: usize = 0;
        let mut delimiters: Vec<usize> = Vec::new();

        for value in raw_record {
            if value == &field_separator {
                delimiters.push(offset);
            }
            offset += 1;
        }

        FieldValues {
            raw_record,
            raw_len: raw_record.len(),
            delimiter_indexes: delimiters,
        }
    }

    fn field_start(&self, field: usize) -> usize {
        if field <= 1 {
            return 0;
        } else if field > &self.delimiter_indexes.len() + 1 {
            return *&self.raw_len;
        }

        &self.delimiter_indexes[field - 2] + 1
    }

    fn field_end(&self, field: usize) -> usize {
        if field > *&self.delimiter_indexes.len() {
            return *&self.raw_len;
        }

        *&self.delimiter_indexes[field - 1]
    }


    fn single(&self, field: usize) -> &[u8] {
        let start = &self.field_start(field);
        let end = &self.field_end(field);
        &self.raw_record[*start..*end]
    }


    fn unbounded(&self, start_field: usize) -> &[u8] {
        let start = *&self.field_start(start_field);
        &self.raw_record[start..]
    }


    fn range(&self, start_field: usize, end_field: usize) -> &[u8] {
        let start = *&self.field_start(start_field);
        let end = *&self.field_end(end_field);

        &self.raw_record[start..end]
    }
}

#[derive(PartialEq, Debug)]
enum Fragment<'a> {
    // "a value" - static string
    StaticValue(&'a [u8]),
    // $2 - field 2
    SingleField(usize),
    // $2,4 - fields 2, 3, 4
    FieldRange(usize, usize),
    // $2,4 - fields 2, 3, 4
    UnboundedFieldRange(usize),
}

#[derive(Debug)]
pub struct OutputTemplate<'a> {
    field_separator: u8,
    record_separator: u8,
    raw_template: &'a str,
    fragments: Vec<Fragment<'a>>,
}

impl PartialEq for OutputTemplate<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.field_separator == other.field_separator
            && self.record_separator == other.record_separator
            && self.raw_template == other.raw_template
            && self.fragments == other.fragments
    }
}

impl OutputTemplate<'_> {
    pub fn parse(raw_template: &str, field_separator: u8, record_separator: u8) -> OutputTemplate {
        let fragments = OutputTemplate::extract_fragments(raw_template);
        OutputTemplate {
            field_separator,
            record_separator,
            raw_template,
            fragments,
        }
    }

    fn write_merged(&self, writer: &mut dyn Write, field_values: FieldValues) -> io::Result<()> {
        self.fragments.iter().for_each(|fragment| {
            let f = match fragment {
                Fragment::StaticValue(val) => val,
                Fragment::SingleField(field) => field_values.single(*field),
                Fragment::UnboundedFieldRange(start_field) => field_values.unbounded(*start_field),
                Fragment::FieldRange(start_field, end_field) => field_values.range(*start_field, *end_field)
            };

            writer.write(&f.as_ref()).ok();
        });
        writer.write(&[NEWLINE_BYTE])?;

        Ok(())
    }

    fn extract_fragments(raw_template: &str) -> Vec<Fragment> {
        let field_placeholder_regex =
            Regex::new(r"\$(?P<start_field>\d+)(?:(?:,(?P<end_field>\d+))|(?P<unbounded>,))?")
                .unwrap();

        let mut fragments = Vec::new();
        let mut last_field_end = 0;
        let template_len = raw_template.len();

        while last_field_end < template_len {
            match field_placeholder_regex.find_at(raw_template, last_field_end) {
                Some(m) => {
                    let field_start = m.start();
                    let field_end = m.end();

                    if last_field_end < field_start {
                        // we have a static string before this field placeholder
                        fragments.push(Fragment::StaticValue(&raw_template[last_field_end..field_start].as_bytes()));
                    }

                    let caps = field_placeholder_regex
                        .captures(&raw_template[field_start..field_end])
                        .unwrap();

                    let start_cap_match = caps.name("start_field").unwrap();

                    let start = *&raw_template[(field_start + start_cap_match.start())
                        ..(field_start + start_cap_match.end())]
                        .parse::<usize>()
                        .unwrap();

                    if caps.name("end_field").is_some() {
                        // field placeholder: $2,4 - fields 2-4
                        let end_cap_match = caps.name("end_field").unwrap();
                        let end_field = *&raw_template[(field_start + end_cap_match.start())
                            ..(field_start + end_cap_match.end())]
                            .parse::<usize>()
                            .unwrap();
                        let natural_start = max(start, 1); // 1 based

                        assert!(end_field > natural_start);

                        fragments.push(Fragment::FieldRange(natural_start, end_field));
                    } else if caps.name("unbounded").is_some() {
                        // field placeholder $2, - all fields 2 and after
                        fragments.push(Fragment::UnboundedFieldRange(start));
                    } else if start == 0 {
                        // field placeholder: $0 - all fields
                        fragments.push(Fragment::UnboundedFieldRange(1));
                    } else {
                        // field placeholder: $2 - just field 2
                        fragments.push(Fragment::SingleField(start));
                    }
                    last_field_end = field_end;
                }
                None => {
                    if last_field_end < template_len {
                        // a static string at the end after the last field_placeholder (or the whole string template)
                        let s = &raw_template[last_field_end..].as_bytes();
                        fragments.push(Fragment::StaticValue(s));
                        last_field_end = template_len;
                    }
                }
            }
        }

        return fragments;
    }
}

pub fn merge_input<R, W>(reader: R, writer: &mut W, template: OutputTemplate) -> io::Result<()> where R: BufRead, W: Write {
    for line_result in reader.split(template.record_separator) {
        let line = line_result?;
        let values = FieldValues::parse(line.as_slice(), template.field_separator);
        template.write_merged(writer, values)?;
    }
    Ok(())
}


#[test]
fn test_merge_single_field() {
    let input: &[u8] = b"first second third fourth fifth sixth\n";
    let cursor = io::Cursor::new(input);

    let mut out = Vec::new();
    merge_input(cursor, &mut out, OutputTemplate::parse("single: $2", SPACE_BYTE, NEWLINE_BYTE)).unwrap();
    assert_eq!(out, b"single: second\n");
}

#[test]
fn test_merge_range() {
    let input: &[u8] = b"first second third fourth fifth sixth\n";
    let cursor = io::Cursor::new(input);

    let mut out = Vec::new();
    merge_input(cursor, &mut out, OutputTemplate::parse("range: $1,3", SPACE_BYTE, NEWLINE_BYTE)).unwrap();
    assert_eq!(out, b"range: first second third\n");
}

#[test]
fn test_merge_unbounded() {
    let input: &[u8] = b"first second third fourth fifth sixth\n";
    let cursor = io::Cursor::new(input);

    let mut out = Vec::new();
    merge_input(cursor, &mut out, OutputTemplate::parse("range: $4,", SPACE_BYTE, NEWLINE_BYTE)).unwrap();
    assert_eq!(out, b"range: fourth fifth sixth\n");
}

#[test]
fn test_merge_all() {
    let input: &[u8] = b"first second third fourth\n";
    let cursor = io::Cursor::new(input);

    let mut out = Vec::new();
    merge_input(cursor, &mut out, OutputTemplate::parse("all: $0", SPACE_BYTE, NEWLINE_BYTE)).unwrap();
    assert_eq!(out, b"all: first second third fourth\n");
}

#[test]
fn test_alternate_field_delimiter() {
    let input: &[u8] = b"first,second,third,fourth,fifth,sixth\n";
    let cursor = io::Cursor::new(input);

    let mut out = Vec::new();
    merge_input(
        cursor,
        &mut out,
        OutputTemplate::parse("single: $2", COMMA_BYTE, NEWLINE_BYTE)).unwrap();
    assert_eq!(out, b"single: second\n");
}

#[test]
fn test_alternate_record_delimiter() {
    let input: &[u8] = b"first,second,third|fourth,fifth,sixth";
    let cursor = io::Cursor::new(input);

    let mut out = Vec::new();
    merge_input(
        cursor,
        &mut out,
        OutputTemplate::parse("single: $2", COMMA_BYTE, PIPE_BYTE)).unwrap();
    assert_eq!(out, b"single: second\nsingle: fifth\n");
}


#[test]
fn test_equality() {
    let a = OutputTemplate {
        field_separator: SPACE_BYTE,
        record_separator: NEWLINE_BYTE,
        raw_template: "baz",
        fragments: vec![
            StaticValue(b""),
            SingleField(1),
            FieldRange(2, 4),
            UnboundedFieldRange(2),
        ],
    };

    let b = OutputTemplate {
        field_separator: SPACE_BYTE,
        record_separator: NEWLINE_BYTE,
        raw_template: "baz",
        fragments: vec![
            StaticValue(b""),
            SingleField(1),
            FieldRange(2, 4),
            UnboundedFieldRange(2),
        ],
    };
    assert_eq!(a, b);
}


#[test]
fn test_extract_fragments() {
    assert_eq!(OutputTemplate::extract_fragments(""), vec![]);
    assert_eq!(
        OutputTemplate::extract_fragments("foo"),
        vec![StaticValue(b"foo")]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("foo bar baz"),
        vec![StaticValue(b"foo bar baz")]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("$1"),
        vec![SingleField(1)]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("foo$1bar"),
        vec![StaticValue(b"foo"), SingleField(1), StaticValue(b"bar")]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("$2,4"),
        vec![FieldRange(2, 4)]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("foo$2,4bar"),
        vec![StaticValue(b"foo"), FieldRange(2, 4), StaticValue(b"bar")]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("$2,"),
        vec![UnboundedFieldRange(2)]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("$1$2,3$0$5,"),
        vec![
            SingleField(1),
            FieldRange(2, 3),
            UnboundedFieldRange(1),
            UnboundedFieldRange(5),
        ]
    );
    assert_eq!(
        OutputTemplate::extract_fragments("foo $1 bar $2,5 baz $6, qux $0 quxx $11,66 quxxx"),
        vec![
            StaticValue(b"foo "),
            SingleField(1),
            StaticValue(b" bar "),
            FieldRange(2, 5),
            StaticValue(b" baz "),
            UnboundedFieldRange(6),
            StaticValue(b" qux "),
            UnboundedFieldRange(1),
            StaticValue(b" quxx "),
            FieldRange(11, 66),
            StaticValue(b" quxxx"),
        ]
    );
}

#[test]
fn test_field_value_extract() {
    let fv = FieldValues::parse(b"one two three", SPACE_BYTE);

    assert_eq!(fv.single(1), b"one");
    assert_eq!(fv.single(2), b"two");
    assert_eq!(fv.single(3), b"three");
    assert_eq!(fv.single(4), b"");

    assert_eq!(fv.range(1, 1), b"one");
    assert_eq!(fv.range(1, 2), b"one two");
    assert_eq!(fv.range(1, 3), b"one two three");
    assert_eq!(fv.range(1, 4), b"one two three");
    assert_eq!(fv.range(2, 4), b"two three");

    assert_eq!(fv.unbounded(1), b"one two three");
    assert_eq!(fv.unbounded(2), b"two three");
    assert_eq!(fv.unbounded(3), b"three");
    assert_eq!(fv.unbounded(4), b"");
}