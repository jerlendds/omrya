use std::collections::HashMap;

use crate::domain::{RyaIri, RyaStatement};
use crate::resolver::{DELIM_BYTE, TYPE_DELIM_BYTE, deserialize, serialize_type};

const ALL_REGEX: &str = "([\\s\\S]*)";
const HASHED_ALL_REGEX: &str = "([0-9a-f]{32})\0";
const LAST_BYTE: u8 = 0xff;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TableLayout {
    Spo,
    Po,
    Osp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TriplePatternStrategyKind {
    SpoWholeRow,
    PoWholeRow,
    NullRow,
    OspWholeRow,
    HashedSpoWholeRow,
    HashedPoWholeRow,
}

impl TriplePatternStrategyKind {
    pub fn layout(self) -> TableLayout {
        match self {
            Self::SpoWholeRow | Self::NullRow | Self::HashedSpoWholeRow => TableLayout::Spo,
            Self::PoWholeRow | Self::HashedPoWholeRow => TableLayout::Po,
            Self::OspWholeRow => TableLayout::Osp,
        }
    }

    pub fn uses_hash_prefix(self) -> bool {
        matches!(self, Self::HashedSpoWholeRow | Self::HashedPoWholeRow)
    }

    pub fn build_regex(
        self,
        subject: Option<&str>,
        predicate: Option<&str>,
        object: Option<&str>,
        context: Option<&str>,
        object_type_info: Option<&[u8]>,
    ) -> Option<TripleRowRegex> {
        if self.uses_hash_prefix() {
            build_hashed_whole_row_regex(
                self.layout(),
                subject,
                predicate,
                object,
                context,
                object_type_info,
            )
        } else {
            build_whole_row_regex(
                self.layout(),
                subject,
                predicate,
                object,
                context,
                object_type_info,
            )
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TripleRow {
    pub row: Vec<u8>,
    pub column_family: Vec<u8>,
    pub column_qualifier: Vec<u8>,
    pub column_visibility: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
    pub timestamp: u64,
}

impl TripleRow {
    fn new(
        row: Vec<u8>,
        column_family: Vec<u8>,
        column_qualifier: Vec<u8>,
        statement: &RyaStatement,
    ) -> Self {
        Self {
            row,
            column_family,
            column_qualifier,
            column_visibility: statement.column_visibility.clone(),
            value: statement.value.clone(),
            timestamp: statement.timestamp,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TripleRowRegex {
    pub row: String,
    pub column_family: Option<String>,
    pub column_qualifier: Option<String>,
}

impl TripleRowRegex {
    pub fn matches_row(&self, row: &[u8]) -> bool {
        let Ok(row) = std::str::from_utf8(row) else {
            return false;
        };
        limited_regex_matches(&self.row, row)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteRange {
    pub start: Vec<u8>,
    pub end: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TripleContext {
    prefix_rows_with_hash: bool,
}

impl TripleContext {
    pub fn new(prefix_rows_with_hash: bool) -> Self {
        Self {
            prefix_rows_with_hash,
        }
    }

    pub fn retrieve_strategy(&self, layout: TableLayout) -> Option<TriplePatternStrategyKind> {
        self.triple_pattern_strategies()
            .into_iter()
            .find(|strategy| strategy.layout() == layout)
    }

    pub fn triple_pattern_strategies(&self) -> Vec<TriplePatternStrategyKind> {
        if self.prefix_rows_with_hash {
            vec![
                TriplePatternStrategyKind::HashedSpoWholeRow,
                TriplePatternStrategyKind::HashedPoWholeRow,
                TriplePatternStrategyKind::OspWholeRow,
            ]
        } else {
            vec![
                TriplePatternStrategyKind::SpoWholeRow,
                TriplePatternStrategyKind::PoWholeRow,
                TriplePatternStrategyKind::NullRow,
                TriplePatternStrategyKind::OspWholeRow,
            ]
        }
    }

    pub fn serialize_triple(
        &self,
        statement: &RyaStatement,
    ) -> Result<HashMap<TableLayout, TripleRow>, String> {
        let subject = statement.subject.data().as_bytes();
        let predicate = statement.predicate.data().as_bytes();
        let (object_data, object_type) = serialize_type(&statement.object)?;
        let column_family = statement
            .context
            .as_ref()
            .map(|context| context.data().as_bytes().to_vec())
            .unwrap_or_default();
        let column_qualifier = statement
            .qualifier
            .as_ref()
            .map(|qualifier| qualifier.as_bytes().to_vec())
            .unwrap_or_default();

        let mut rows = HashMap::new();
        rows.insert(
            TableLayout::Spo,
            TripleRow::new(
                self.spo_row(subject, predicate, &object_data, &object_type),
                column_family.clone(),
                column_qualifier.clone(),
                statement,
            ),
        );
        rows.insert(
            TableLayout::Po,
            TripleRow::new(
                self.po_row(subject, predicate, &object_data, &object_type),
                column_family.clone(),
                column_qualifier.clone(),
                statement,
            ),
        );
        rows.insert(
            TableLayout::Osp,
            TripleRow::new(
                concat_parts(&[
                    &object_data,
                    &[DELIM_BYTE],
                    subject,
                    &[DELIM_BYTE],
                    predicate,
                    &object_type,
                ]),
                column_family,
                column_qualifier,
                statement,
            ),
        );
        Ok(rows)
    }

    pub fn deserialize_triple(
        &self,
        layout: TableLayout,
        triple_row: &TripleRow,
    ) -> Result<RyaStatement, String> {
        let row =
            if self.prefix_rows_with_hash && matches!(layout, TableLayout::Spo | TableLayout::Po) {
                let hash_end = triple_row
                    .row
                    .iter()
                    .position(|byte| *byte == DELIM_BYTE)
                    .ok_or_else(|| "Hashed row is missing hash delimiter".to_string())?;
                &triple_row.row[hash_end + 1..]
            } else {
                &triple_row.row
            };

        let first_index = row
            .iter()
            .position(|byte| *byte == DELIM_BYTE)
            .ok_or_else(|| "Triple row is missing first delimiter".to_string())?;
        let second_index = row
            .iter()
            .rposition(|byte| *byte == DELIM_BYTE)
            .ok_or_else(|| "Triple row is missing second delimiter".to_string())?;
        let type_index = row
            .iter()
            .position(|byte| *byte == TYPE_DELIM_BYTE)
            .ok_or_else(|| "Triple row is missing type delimiter".to_string())?;

        let first = &row[..first_index];
        let second = &row[first_index + 1..second_index];
        let third = &row[second_index + 1..type_index];
        let type_suffix = &row[type_index..];
        let object_bytes = match layout {
            TableLayout::Spo => concat_parts(&[third, type_suffix]),
            TableLayout::Po => concat_parts(&[second, type_suffix]),
            TableLayout::Osp => concat_parts(&[first, type_suffix]),
        };
        let object = deserialize(&object_bytes)?;

        let (subject, predicate) = match layout {
            TableLayout::Spo => (bytes_to_iri(first)?, bytes_to_iri(second)?),
            TableLayout::Po => (bytes_to_iri(third)?, bytes_to_iri(first)?),
            TableLayout::Osp => (bytes_to_iri(second)?, bytes_to_iri(third)?),
        };

        let mut statement =
            RyaStatement::new(subject, predicate, object).with_timestamp(triple_row.timestamp);
        statement.context = if triple_row.column_family.is_empty() {
            None
        } else {
            Some(bytes_to_iri(&triple_row.column_family)?)
        };
        statement.qualifier = if triple_row.column_qualifier.is_empty() {
            None
        } else {
            Some(
                String::from_utf8(triple_row.column_qualifier.clone())
                    .map_err(|e| e.to_string())?,
            )
        };
        statement.column_visibility = triple_row.column_visibility.clone();
        statement.value = triple_row.value.clone();
        Ok(statement)
    }

    fn spo_row(
        &self,
        subject: &[u8],
        predicate: &[u8],
        object_data: &[u8],
        object_type: &[u8],
    ) -> Vec<u8> {
        let row = concat_parts(&[
            subject,
            &[DELIM_BYTE],
            predicate,
            &[DELIM_BYTE],
            object_data,
            object_type,
        ]);
        if self.prefix_rows_with_hash {
            prefix_hash(subject, row)
        } else {
            row
        }
    }

    fn po_row(
        &self,
        subject: &[u8],
        predicate: &[u8],
        object_data: &[u8],
        object_type: &[u8],
    ) -> Vec<u8> {
        let row = concat_parts(&[
            predicate,
            &[DELIM_BYTE],
            object_data,
            &[DELIM_BYTE],
            subject,
            object_type,
        ]);
        if self.prefix_rows_with_hash {
            prefix_hash(predicate, row)
        } else {
            row
        }
    }
}

fn bytes_to_iri(bytes: &[u8]) -> Result<RyaIri, String> {
    let data = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
    RyaIri::new(data)
}

fn prefix_hash(seed: &[u8], row: Vec<u8>) -> Vec<u8> {
    let mut prefixed = md5_hex(seed).into_bytes();
    prefixed.push(DELIM_BYTE);
    prefixed.extend(row);
    prefixed
}

pub fn build_whole_row_regex(
    layout: TableLayout,
    subject: Option<&str>,
    predicate: Option<&str>,
    object: Option<&str>,
    context: Option<&str>,
    object_type_info: Option<&[u8]>,
) -> Option<TripleRowRegex> {
    if subject.is_none()
        && predicate.is_none()
        && object.is_none()
        && context.is_none()
        && object_type_info.is_none()
    {
        return None;
    }

    let (first, second, third) = match layout {
        TableLayout::Spo => (subject, predicate, object),
        TableLayout::Po => (predicate, object, subject),
        TableLayout::Osp => (object, subject, predicate),
    };
    Some(TripleRowRegex {
        row: build_row_regex("", first, second, third, object_type_info),
        column_family: context.map(|value| format!("{value}{ALL_REGEX}")),
        column_qualifier: None,
    })
}

pub fn build_hashed_whole_row_regex(
    layout: TableLayout,
    subject: Option<&str>,
    predicate: Option<&str>,
    object: Option<&str>,
    context: Option<&str>,
    object_type_info: Option<&[u8]>,
) -> Option<TripleRowRegex> {
    if matches!(layout, TableLayout::Osp) {
        return build_whole_row_regex(
            layout,
            subject,
            predicate,
            object,
            context,
            object_type_info,
        );
    }
    if subject.is_none()
        && predicate.is_none()
        && object.is_none()
        && context.is_none()
        && object_type_info.is_none()
    {
        return None;
    }

    let (first, second, third) = match layout {
        TableLayout::Spo => (subject, predicate, object),
        TableLayout::Po => (predicate, object, subject),
        TableLayout::Osp => unreachable!(),
    };
    Some(TripleRowRegex {
        row: build_row_regex(HASHED_ALL_REGEX, first, second, third, object_type_info),
        column_family: context.map(|value| format!("{value}{ALL_REGEX}")),
        column_qualifier: None,
    })
}

pub fn hashed_subject_range(subject: &RyaIri) -> ByteRange {
    hashed_prefix_range(subject.data().as_bytes())
}

pub fn hashed_predicate_range(predicate: &RyaIri) -> ByteRange {
    hashed_prefix_range(predicate.data().as_bytes())
}

fn hashed_prefix_range(value: &[u8]) -> ByteRange {
    let mut start = md5_hex(value).into_bytes();
    start.push(DELIM_BYTE);
    start.extend_from_slice(value);
    start.push(DELIM_BYTE);

    let mut end = start.clone();
    end.push(LAST_BYTE);
    ByteRange { start, end }
}

fn build_row_regex(
    prefix: &str,
    first: Option<&str>,
    second: Option<&str>,
    third: Option<&str>,
    object_type_info: Option<&[u8]>,
) -> String {
    let mut row = String::from(prefix);
    row.push_str(first.unwrap_or(ALL_REGEX));
    row.push(DELIM_BYTE as char);
    row.push_str(second.unwrap_or(ALL_REGEX));
    row.push(DELIM_BYTE as char);

    if let Some(third) = third {
        row.push_str(third);
        if let Some(object_type_info) = object_type_info {
            row.push_str(&String::from_utf8_lossy(object_type_info));
        } else {
            row.push(TYPE_DELIM_BYTE as char);
            row.push_str(ALL_REGEX);
        }
    } else {
        row.push_str(ALL_REGEX);
        if let Some(object_type_info) = object_type_info {
            row.push_str(&String::from_utf8_lossy(object_type_info));
        }
    }

    row
}

fn md5_hex(input: &[u8]) -> String {
    let digest = md5_digest(input);
    let mut out = String::with_capacity(32);
    for byte in digest {
        out.push(hex_char(byte >> 4));
        out.push(hex_char(byte & 0x0f));
    }
    out
}

fn hex_char(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + nibble - 10) as char,
        _ => unreachable!(),
    }
}

fn md5_digest(input: &[u8]) -> [u8; 16] {
    let mut message = input.to_vec();
    let bit_len = (message.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_le_bytes());

    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 16];
        for (word, bytes) in words.iter_mut().zip(chunk.chunks_exact(4)) {
            *word = u32::from_le_bytes(bytes.try_into().unwrap());
        }

        let mut a = a0;
        let mut b = b0;
        let mut c = c0;
        let mut d = d0;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };
            let next = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(words[g])
                    .rotate_left(S[i]),
            );
            a = next;
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut digest = [0u8; 16];
    digest[..4].copy_from_slice(&a0.to_le_bytes());
    digest[4..8].copy_from_slice(&b0.to_le_bytes());
    digest[8..12].copy_from_slice(&c0.to_le_bytes());
    digest[12..16].copy_from_slice(&d0.to_le_bytes());
    digest
}

fn concat_parts(parts: &[&[u8]]) -> Vec<u8> {
    let len = parts.iter().map(|part| part.len()).sum();
    let mut bytes = Vec::with_capacity(len);
    for part in parts {
        bytes.extend_from_slice(part);
    }
    bytes
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RegexToken {
    Literal(char),
    AnySequence,
    ClassRepeat(Vec<char>, usize),
}

fn limited_regex_matches(pattern: &str, value: &str) -> bool {
    let tokens = parse_limited_regex(pattern);
    let chars = value.chars().collect::<Vec<_>>();
    match_tokens(&tokens, &chars, 0, 0)
}

fn parse_limited_regex(pattern: &str) -> Vec<RegexToken> {
    let chars = pattern.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if starts_with_chars(&chars[index..], "([\\s\\S]*)") {
            tokens.push(RegexToken::AnySequence);
            index += "([\\s\\S]*)".chars().count();
        } else if chars[index] == '(' {
            if let Some((token, next)) = parse_grouped_class_repeat(&chars, index) {
                tokens.push(token);
                index = next;
            } else {
                tokens.push(RegexToken::Literal(chars[index]));
                index += 1;
            }
        } else if chars[index] == '[' {
            if let Some((class, next)) = parse_char_class(&chars, index) {
                let (count, after_count) = parse_repeat_count(&chars, next).unwrap_or((1, next));
                tokens.push(RegexToken::ClassRepeat(class, count));
                index = after_count;
            } else {
                tokens.push(RegexToken::Literal(chars[index]));
                index += 1;
            }
        } else {
            tokens.push(RegexToken::Literal(chars[index]));
            index += 1;
        }
    }
    tokens
}

fn starts_with_chars(chars: &[char], needle: &str) -> bool {
    let needle = needle.chars().collect::<Vec<_>>();
    chars.len() >= needle.len() && chars[..needle.len()] == needle
}

fn parse_grouped_class_repeat(chars: &[char], start: usize) -> Option<(RegexToken, usize)> {
    if chars.get(start) != Some(&'(') || chars.get(start + 1) != Some(&'[') {
        return None;
    }
    let (class, after_class) = parse_char_class(chars, start + 1)?;
    let (count, after_count) = parse_repeat_count(chars, after_class)?;
    if chars.get(after_count) != Some(&')') {
        return None;
    }
    Some((RegexToken::ClassRepeat(class, count), after_count + 1))
}

fn parse_char_class(chars: &[char], start: usize) -> Option<(Vec<char>, usize)> {
    if chars.get(start) != Some(&'[') {
        return None;
    }
    let mut class = Vec::new();
    let mut index = start + 1;
    while index < chars.len() && chars[index] != ']' {
        if index + 2 < chars.len() && chars[index + 1] == '-' && chars[index + 2] != ']' {
            let begin = chars[index] as u32;
            let end = chars[index + 2] as u32;
            for codepoint in begin..=end {
                if let Some(ch) = char::from_u32(codepoint) {
                    class.push(ch);
                }
            }
            index += 3;
        } else {
            class.push(chars[index]);
            index += 1;
        }
    }
    if index >= chars.len() || chars[index] != ']' || class.is_empty() {
        return None;
    }
    Some((class, index + 1))
}

fn parse_repeat_count(chars: &[char], start: usize) -> Option<(usize, usize)> {
    if chars.get(start) != Some(&'{') {
        return None;
    }
    let mut index = start + 1;
    let mut digits = String::new();
    while index < chars.len() && chars[index].is_ascii_digit() {
        digits.push(chars[index]);
        index += 1;
    }
    if digits.is_empty() || chars.get(index) != Some(&'}') {
        return None;
    }
    Some((digits.parse().ok()?, index + 1))
}

fn match_tokens(
    tokens: &[RegexToken],
    chars: &[char],
    token_index: usize,
    char_index: usize,
) -> bool {
    if token_index == tokens.len() {
        return char_index == chars.len();
    }

    match &tokens[token_index] {
        RegexToken::Literal(ch) => {
            chars.get(char_index) == Some(ch)
                && match_tokens(tokens, chars, token_index + 1, char_index + 1)
        }
        RegexToken::ClassRepeat(class, count) => {
            if char_index + count > chars.len() {
                return false;
            }
            chars[char_index..char_index + count]
                .iter()
                .all(|ch| class.contains(ch))
                && match_tokens(tokens, chars, token_index + 1, char_index + count)
        }
        RegexToken::AnySequence => (char_index..=chars.len())
            .any(|next_index| match_tokens(tokens, chars, token_index + 1, next_index)),
    }
}

#[cfg(test)]
#[path = "../tests/resolver_triple_tests.rs"]
mod tests;
