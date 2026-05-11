use std::collections::BTreeMap;

pub const FAMILY_DELIM: u8 = 0;
pub const FAMILY_DELIM_STR: &str = "\0";
pub const INDEX_DELIM: u8 = 1;
pub const INDEX_DELIM_STR: &str = "\x01";
pub const URI_MARKER: u8 = 7;
pub const URI_MARKER_STR: &str = "\x07";
pub const BNODE_MARKER: u8 = 8;
pub const PLAIN_LITERAL_MARKER: u8 = 9;
pub const PLAIN_LITERAL_MARKER_STR: &str = "\x09";
pub const LANG_LITERAL_MARKER: u8 = 10;
pub const DATATYPE_LITERAL_MARKER: u8 = 11;
pub const DATATYPE_LITERAL_MARKER_STR: &str = "\x0b";

pub const DEFAULT_PAIR_DELIMITER: &str = "\0";
pub const DEFAULT_VALUE_DELIMITER: &str = "\u{fffd}";

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct PartitionKey {
    pub row: String,
    pub column_family: String,
    pub column_qualifier: String,
}

impl PartitionKey {
    pub fn new(
        row: impl Into<String>,
        column_family: impl Into<String>,
        column_qualifier: impl Into<String>,
    ) -> Self {
        Self {
            row: row.into(),
            column_family: column_family.into(),
            column_qualifier: column_qualifier.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudbaseValueConverter {
    pair_delimiter: String,
    value_delimiter: String,
}

impl Default for CloudbaseValueConverter {
    fn default() -> Self {
        Self {
            pair_delimiter: DEFAULT_PAIR_DELIMITER.to_string(),
            value_delimiter: DEFAULT_VALUE_DELIMITER.to_string(),
        }
    }
}

impl CloudbaseValueConverter {
    pub fn new(pair_delimiter: Option<&str>, value_delimiter: Option<&str>) -> Self {
        Self {
            pair_delimiter: non_empty_or(pair_delimiter, DEFAULT_PAIR_DELIMITER).to_string(),
            value_delimiter: non_empty_or(value_delimiter, DEFAULT_VALUE_DELIMITER).to_string(),
        }
    }

    pub fn to_map(&self, value: &str) -> BTreeMap<String, String> {
        parse_record(value, &self.pair_delimiter, &self.value_delimiter)
    }

    pub fn to_value(&self, record: &BTreeMap<String, String>) -> String {
        let mut out = String::new();
        for (index, (key, value)) in record.iter().enumerate() {
            if index > 0 {
                out.push_str(&self.pair_delimiter);
            }
            out.push_str(key);
            out.push_str(&self.value_delimiter);
            out.push_str(value);
        }
        out
    }
}

fn parse_record(
    value: &str,
    pair_delimiter: &str,
    value_delimiter: &str,
) -> BTreeMap<String, String> {
    let mut row = BTreeMap::new();
    let mut value_start = 0usize;
    let value_end = value.len();
    let pair_len = pair_delimiter.len();
    let value_len = value_delimiter.len();

    while value_start < value_end {
        let Some(v_index) = find_from(value, value_delimiter, value_start) else {
            break;
        };

        let key = value[value_start..v_index].trim().to_string();
        let next_value_delim =
            find_from(value, value_delimiter, v_index + value_len).unwrap_or(value_end);
        let next_pair_delim =
            find_from(value, pair_delimiter, v_index + value_len).unwrap_or(value_end);
        let field_end = next_value_delim.min(next_pair_delim);

        let val_start = v_index + value_len;
        let val = value[val_start..field_end].trim().to_string();
        row.insert(key, val);

        if next_pair_delim == value_end {
            break;
        }
        value_start = next_pair_delim + pair_len;
    }

    row
}

fn find_from(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    haystack
        .get(from..)
        .and_then(|tail| tail.find(needle).map(|index| from + index))
}

fn non_empty_or<'a>(value: Option<&'a str>, default: &'a str) -> &'a str {
    match value {
        Some(value) if !value.is_empty() => value,
        _ => default,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversionOperation {
    field: String,
    op: char,
    operand: f64,
}

impl ConversionOperation {
    pub fn parse(config: &str) -> Result<Self, String> {
        let config = config.strip_prefix("conversion.").unwrap_or(config);
        let parts = config.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(format!("'{config}' was not in the format 'field op value'"));
        }
        let op = parts[1]
            .chars()
            .next()
            .ok_or_else(|| "Missing operator".to_string())?;
        if !['+', '-', '*', '/', '%', '^'].contains(&op) {
            return Err(format!(
                "Operator '{op}' is not among the supported operators: +,-,*,/,%,^"
            ));
        }
        let operand = parts[2]
            .parse::<f64>()
            .map_err(|_| format!("Operand '{}' could not be parsed as a number.", parts[2]))?;
        Ok(Self {
            field: parts[0].to_string(),
            op,
            operand,
        })
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn execute(&self, value: Option<&str>) -> Option<String> {
        let value = value?;
        let mut parsed = value
            .parse::<f64>()
            .or_else(|_| i64::from_str_radix(value, 16).map(|n| n as f64))
            .ok()?;
        match self.op {
            '+' => parsed += self.operand,
            '-' => parsed -= self.operand,
            '*' => parsed *= self.operand,
            '/' => parsed /= self.operand,
            '%' => parsed %= self.operand,
            '^' => parsed = parsed.powf(self.operand),
            _ => {}
        }
        Some(java_double_string(parsed))
    }
}

fn java_double_string(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GvFrequencyFilter {
    frequency: String,
    converter: CloudbaseValueConverter,
}

impl GvFrequencyFilter {
    pub fn new(frequency: Option<&str>) -> Self {
        Self {
            frequency: non_empty_or(frequency, "0.0").to_string(),
            converter: CloudbaseValueConverter::default(),
        }
    }

    pub fn accept_value(&self, value: &str) -> bool {
        let Ok(freq) = self.frequency.parse::<f64>() else {
            return false;
        };
        let record = self.converter.to_map(value);
        let Some(center) = record.get("frequency").and_then(|v| v.parse::<f64>().ok()) else {
            return false;
        };
        let Some(bandwidth) = record.get("bandwidth").and_then(|v| v.parse::<f64>().ok()) else {
            return false;
        };
        center - 0.5 * bandwidth <= freq && freq <= center + 0.5 * bandwidth
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GvDateFilter {
    timestamp_millis: i64,
    date_start_field: String,
    date_end_field: String,
    active_field: String,
    converter: CloudbaseValueConverter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeRangeFilter {
    time_range: i64,
    start_time: i64,
}

impl TimeRangeFilter {
    pub const TIME_RANGE_PROP: &'static str = "timeRange";
    pub const START_TIME_PROP: &'static str = "startTime";

    pub fn new(time_range: i64, start_time: i64) -> Self {
        Self {
            time_range,
            start_time,
        }
    }

    pub fn from_options(
        options: &BTreeMap<String, String>,
        current_time_millis: i64,
    ) -> Result<Self, String> {
        let time_range = options
            .get(Self::TIME_RANGE_PROP)
            .ok_or_else(|| "timeRange must be set for TimeRangeFilter".to_string())?
            .parse::<i64>()
            .map_err(|e| format!("Invalid TimeRangeFilter timeRange: {e}"))?;
        let start_time = options
            .get(Self::START_TIME_PROP)
            .map(|value| {
                value
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid TimeRangeFilter startTime: {e}"))
            })
            .transpose()?
            .unwrap_or(current_time_millis);

        Ok(Self::new(time_range, start_time))
    }

    pub fn accept_timestamp(&self, timestamp: i64) -> bool {
        let diff = self.start_time - timestamp;
        !(diff > self.time_range || diff < 0)
    }
}

impl GvDateFilter {
    pub fn new(timestamp: Option<&str>) -> Self {
        let timestamp = non_empty_or(timestamp, "2011-03-03T20:44:28.633Z");
        Self {
            timestamp_millis: parse_gv_date_millis(timestamp).unwrap_or(0),
            date_start_field: "date-start".to_string(),
            date_end_field: "date-end".to_string(),
            active_field: "version".to_string(),
            converter: CloudbaseValueConverter::default(),
        }
    }

    pub fn accept_value(&self, value: &str) -> bool {
        let record = self.converter.to_map(value);
        if record.get(&self.active_field).is_some_and(|v| v == "0") {
            return false;
        }

        let start = record
            .get(&self.date_start_field)
            .and_then(|v| parse_gv_date_millis(v).ok());
        let end = record
            .get(&self.date_end_field)
            .and_then(|v| parse_gv_date_millis(v).ok());

        match (start, end) {
            (Some(start), Some(end)) => {
                start < self.timestamp_millis && self.timestamp_millis < end
            }
            (Some(start), None) => start < self.timestamp_millis,
            _ => false,
        }
    }
}

fn parse_gv_date_millis(input: &str) -> Result<i64, String> {
    let year = parse_i64(input, 0, 4)?;
    let month = parse_i64(input, 5, 7)?;
    let day = parse_i64(input, 8, 10)?;
    if input.len() == 10 {
        return Ok(days_from_civil(year, month, day) * 86_400_000);
    }
    if input.len() < 20 || &input[10..11] != "T" {
        return Err(format!("Unsupported date format: {input}"));
    }
    let hour = parse_i64(input, 11, 13)?;
    let minute = parse_i64(input, 14, 16)?;
    let second = parse_i64(input, 17, 19)?;
    let mut millis = 0;
    let mut tz_start = 19;
    if input.as_bytes().get(19) == Some(&b'.') {
        let tail = &input[20..];
        let digits = tail
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        tz_start = 20 + digits.len();
        millis = format!("{digits:0<3}")
            .get(..3)
            .unwrap_or("000")
            .parse::<i64>()
            .map_err(|e| e.to_string())?;
    }
    if input.get(tz_start..) != Some("Z") {
        return Err(format!("Unsupported date timezone: {input}"));
    }
    Ok(days_from_civil(year, month, day) * 86_400_000
        + hour * 3_600_000
        + minute * 60_000
        + second * 1_000
        + millis)
}

fn parse_i64(input: &str, start: usize, end: usize) -> Result<i64, String> {
    input[start..end].parse::<i64>().map_err(|e| e.to_string())
}

fn days_from_civil(mut year: i64, month: i64, day: i64) -> i64 {
    year -= (month <= 2) as i64;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[derive(Clone, Debug, PartialEq)]
pub struct OgcFilter {
    root: OgcOperation,
    pair_delimiter: String,
    value_delimiter: String,
    colf_name: Option<String>,
    colq_name: Option<String>,
}

impl OgcFilter {
    pub fn parse(filter_xml: &str) -> Result<Self, String> {
        let node = XmlNode::parse(filter_xml)?;
        let root = if node.name.eq_ignore_ascii_case("filter") {
            node.children
                .iter()
                .find(|child| !child.name.starts_with('#'))
                .ok_or_else(|| "Filter element did not contain an operation".to_string())?
        } else {
            &node
        };
        Ok(Self {
            root: OgcOperation::from_xml(root, CompareType::Auto)?,
            pair_delimiter: DEFAULT_PAIR_DELIMITER.to_string(),
            value_delimiter: DEFAULT_VALUE_DELIMITER.to_string(),
            colf_name: None,
            colq_name: None,
        })
    }

    pub fn accept(&self, key: &PartitionKey, value: &str) -> bool {
        let mut row = parse_record(value, &self.pair_delimiter, &self.value_delimiter);
        if let Some(colf_name) = &self.colf_name {
            row.insert(colf_name.clone(), key.column_family.clone());
        }
        if let Some(colq_name) = &self.colq_name {
            row.insert(colq_name.clone(), key.column_qualifier.clone());
        }
        self.accept_record(&row)
    }

    pub fn accept_record(&self, record: &BTreeMap<String, String>) -> bool {
        self.root.execute(record)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompareType {
    Auto,
}

#[derive(Clone, Debug, PartialEq)]
enum OgcOperation {
    And(Vec<OgcOperation>),
    Or(Vec<OgcOperation>),
    Not(Vec<OgcOperation>),
    Equal(Comparison),
    NotEqual(Comparison),
    GreaterThan(Comparison),
    GreaterThanOrEqual(Comparison),
    LessThan(Comparison),
    LessThanOrEqual(Comparison),
    Between {
        name: String,
        lower: String,
        upper: String,
        numeric: Option<(f64, f64)>,
    },
    Like {
        name: String,
        pattern: LikePattern,
    },
    Null {
        name: String,
    },
    BBox(BBox),
}

impl OgcOperation {
    fn from_xml(node: &XmlNode, compare_type: CompareType) -> Result<Self, String> {
        match node.name.as_str() {
            "And" => Ok(Self::And(parse_children(node, compare_type)?)),
            "Or" => Ok(Self::Or(parse_children(node, compare_type)?)),
            "Not" => Ok(Self::Not(parse_children(node, compare_type)?)),
            "PropertyIsEqualTo" => Ok(Self::Equal(Comparison::from_xml(node, compare_type)?)),
            "PropertyIsNotEqualTo" => Ok(Self::NotEqual(Comparison::from_xml(node, compare_type)?)),
            "PropertyIsGreaterThan" => {
                Ok(Self::GreaterThan(Comparison::from_xml(node, compare_type)?))
            }
            "PropertyIsGreaterThanOrEqualTo" => Ok(Self::GreaterThanOrEqual(Comparison::from_xml(
                node,
                compare_type,
            )?)),
            "PropertyIsLessThan" => Ok(Self::LessThan(Comparison::from_xml(node, compare_type)?)),
            "PropertyIsLessThanOrEqualTo" => Ok(Self::LessThanOrEqual(Comparison::from_xml(
                node,
                compare_type,
            )?)),
            "PropertyIsBetween" => {
                let name = text_of(node, "PropertyName").unwrap_or_default();
                let lower = text_of(node, "LowerBoundary").unwrap_or_default();
                let upper = text_of(node, "UpperBoundary").unwrap_or_default();
                let numeric = lower.parse::<f64>().ok().zip(upper.parse::<f64>().ok());
                Ok(Self::Between {
                    name,
                    lower,
                    upper,
                    numeric,
                })
            }
            "PropertyIsLike" => {
                let name = text_of(node, "PropertyName").unwrap_or_default();
                let literal = text_of(node, "Literal").unwrap_or_default();
                Ok(Self::Like {
                    name,
                    pattern: LikePattern::from_ogc(&literal),
                })
            }
            "PropertyIsNull" => Ok(Self::Null {
                name: text_of(node, "PropertyName").unwrap_or_else(|| node.text.clone()),
            }),
            "BBOX" => Ok(Self::BBox(BBox::from_xml(node)?)),
            other => Err(format!("Operation not supported: {other}")),
        }
    }

    fn execute(&self, row: &BTreeMap<String, String>) -> bool {
        match self {
            Self::And(children) => children.iter().all(|child| child.execute(row)),
            Self::Or(children) => children.iter().any(|child| child.execute(row)),
            Self::Not(children) => children.iter().all(|child| !child.execute(row)),
            Self::Equal(comparison) => comparison.compare(row, |a, b| a == b, |a, b| a == b),
            Self::NotEqual(comparison) => comparison.compare(row, |a, b| a != b, |a, b| a != b),
            Self::GreaterThan(comparison) => comparison.compare(row, |a, b| a > b, |a, b| a > b),
            Self::GreaterThanOrEqual(comparison) => {
                comparison.compare(row, |a, b| a >= b, |a, b| a >= b)
            }
            Self::LessThan(comparison) => comparison.compare(row, |a, b| a < b, |a, b| a < b),
            Self::LessThanOrEqual(comparison) => {
                comparison.compare(row, |a, b| a <= b, |a, b| a <= b)
            }
            Self::Between {
                name,
                lower,
                upper,
                numeric,
            } => {
                let value = row.get(name).map(String::as_str).unwrap_or("");
                if let Some((lower, upper)) = numeric
                    && let Ok(parsed) = value.parse::<f64>()
                {
                    return *lower <= parsed && parsed <= *upper;
                }
                lower.as_str() <= value && value <= upper.as_str()
            }
            Self::Like { name, pattern } => {
                pattern.matches(row.get(name).map(String::as_str).unwrap_or(""))
            }
            Self::Null { name } => row.get(name).is_none_or(|value| value.is_empty()),
            Self::BBox(bbox) => bbox.contains_row(row),
        }
    }
}

fn parse_children(node: &XmlNode, compare_type: CompareType) -> Result<Vec<OgcOperation>, String> {
    node.children
        .iter()
        .map(|child| OgcOperation::from_xml(child, compare_type))
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
struct Comparison {
    name: String,
    literal: String,
    literal_num: Option<f64>,
}

impl Comparison {
    fn from_xml(node: &XmlNode, _compare_type: CompareType) -> Result<Self, String> {
        let name = text_of(node, "PropertyName").unwrap_or_default();
        let literal = text_of(node, "Literal").unwrap_or_default();
        let literal_num = parse_numeric(&literal);
        Ok(Self {
            name,
            literal,
            literal_num,
        })
    }

    fn compare(
        &self,
        row: &BTreeMap<String, String>,
        numeric_cmp: impl Fn(f64, f64) -> bool,
        string_cmp: impl Fn(&str, &str) -> bool,
    ) -> bool {
        let value = row.get(&self.name).map(String::as_str).unwrap_or("");
        if let Some(literal_num) = self.literal_num
            && let Some(value_num) = parse_numeric(value)
        {
            return numeric_cmp(value_num, literal_num);
        }
        string_cmp(value, &self.literal)
    }
}

fn parse_numeric(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .or_else(|| value.parse::<i64>().ok().map(|v| v as f64))
}

#[derive(Clone, Debug, PartialEq)]
struct LikePattern {
    parts: Vec<String>,
    starts_with_wildcard: bool,
}

impl LikePattern {
    fn from_ogc(pattern: &str) -> Self {
        Self {
            parts: pattern
                .split('*')
                .filter(|part| !part.is_empty())
                .map(|part| part.to_ascii_lowercase())
                .collect(),
            starts_with_wildcard: pattern.starts_with('*'),
        }
    }

    fn matches(&self, value: &str) -> bool {
        let value = value.to_ascii_lowercase();
        if self.parts.is_empty() {
            return true;
        }
        let mut cursor = 0usize;
        for (index, part) in self.parts.iter().enumerate() {
            let Some(found) = value[cursor..].find(part) else {
                return false;
            };
            if index == 0 && !self.starts_with_wildcard && found != 0 {
                return false;
            }
            cursor += found + part.len();
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
struct BBox {
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
}

impl BBox {
    fn from_xml(node: &XmlNode) -> Result<Self, String> {
        let envelope = node
            .children
            .iter()
            .find(|child| child.name.eq_ignore_ascii_case("gml:Envelope"))
            .ok_or_else(|| "BBOX missing gml:Envelope".to_string())?;
        let mut lon_lat = Vec::new();
        for child in &envelope.children {
            let parts = child
                .text
                .split_whitespace()
                .filter_map(|part| part.parse::<f64>().ok())
                .collect::<Vec<_>>();
            if parts.len() == 2 {
                lon_lat.push((parts[0], parts[1]));
            }
        }
        if lon_lat.len() < 2 {
            return Err("BBOX envelope missing corners".to_string());
        }
        let min_lon = lon_lat
            .iter()
            .map(|(lon, _)| *lon)
            .fold(f64::INFINITY, f64::min);
        let max_lon = lon_lat
            .iter()
            .map(|(lon, _)| *lon)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_lat = lon_lat
            .iter()
            .map(|(_, lat)| *lat)
            .fold(f64::INFINITY, f64::min);
        let max_lat = lon_lat
            .iter()
            .map(|(_, lat)| *lat)
            .fold(f64::NEG_INFINITY, f64::max);
        Ok(Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        })
    }

    fn contains_row(&self, row: &BTreeMap<String, String>) -> bool {
        let Some(lon) = first_degree(row, &["lonself", "lon", "long", "longitude"]) else {
            return false;
        };
        let Some(lat) = first_degree(row, &["latself", "lat", "latitude"]) else {
            return false;
        };
        let mut lon = lon;
        while lon < self.min_lon {
            lon += 360.0;
        }
        while lon > self.max_lon {
            lon -= 360.0;
        }
        self.min_lon <= lon && lon <= self.max_lon && self.min_lat <= lat && lat <= self.max_lat
    }
}

fn first_degree(row: &BTreeMap<String, String>, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|name| {
        row.get(*name)
            .filter(|v| v.as_str() != "-")?
            .parse::<f64>()
            .ok()
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct XmlNode {
    name: String,
    text: String,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn parse(input: &str) -> Result<Self, String> {
        let mut parser = XmlParser { input, cursor: 0 };
        parser.parse_node()
    }
}

struct XmlParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl XmlParser<'_> {
    fn parse_node(&mut self) -> Result<XmlNode, String> {
        self.skip_ws();
        self.expect('<')?;
        let raw_name = self.read_until(&['>', ' '])?;
        while self.peek() != Some('>') {
            self.bump();
        }
        self.expect('>')?;

        let name = raw_name.split_whitespace().next().unwrap_or("").to_string();
        let mut text = String::new();
        let mut children = Vec::new();
        loop {
            self.skip_ws();
            if self.starts_with("</") {
                self.cursor += 2;
                let close = self.read_until(&['>'])?;
                self.expect('>')?;
                if close != name {
                    return Err(format!("Expected closing tag for {name}, got {close}"));
                }
                break;
            }
            if self.peek() == Some('<') {
                children.push(self.parse_node()?);
            } else {
                text.push_str(&self.read_until(&['<'])?);
            }
        }
        Ok(XmlNode {
            name,
            text: text.trim().to_string(),
            children,
        })
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.input[self.cursor..].starts_with(prefix)
    }

    fn read_until(&mut self, stops: &[char]) -> Result<String, String> {
        let start = self.cursor;
        while let Some(ch) = self.peek() {
            if stops.contains(&ch) {
                return Ok(self.input[start..self.cursor].trim().to_string());
            }
            self.bump();
        }
        Err("Unexpected end of XML".to_string())
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.peek() {
            Some(ch) if ch == expected => {
                self.bump();
                Ok(())
            }
            _ => Err(format!("Expected XML char {expected}")),
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.cursor += ch.len_utf8();
        }
    }
}

fn text_of(node: &XmlNode, name: &str) -> Option<String> {
    node.children
        .iter()
        .find(|child| child.name.eq_ignore_ascii_case(name))
        .map(|child| {
            if child.text.is_empty() && !child.children.is_empty() {
                child.children[0].text.clone()
            } else {
                child.text.clone()
            }
        })
}

pub fn calculate_end_location(
    lon: f64,
    lat: f64,
    distance_km: f64,
    bearing_degrees: f64,
) -> Option<(f64, f64)> {
    let radius_km = 6371.0f64;
    let lon1 = lon.to_radians();
    let lat1 = lat.to_radians();
    let bearing = bearing_degrees.to_radians();
    let angular = distance_km / radius_km;

    let lat2 = (lat1.sin() * angular.cos() + lat1.cos() * angular.sin() * bearing.cos()).asin();
    let lon2 = lon1
        + (bearing.sin() * angular.sin() * lat1.cos())
            .atan2(angular.cos() - lat1.sin() * lat2.sin());
    let lon2 =
        (lon2 + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI;
    if lat2.is_nan() || lon2.is_nan() {
        return None;
    }
    Some((lon2.to_degrees(), lat2.to_degrees()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RdfValue {
    Iri(String),
    BNode(String),
    PlainLiteral(String),
    LangLiteral { label: String, language: String },
    DatatypeLiteral { label: String, datatype: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdfStatement {
    pub subject: RdfValue,
    pub predicate: RdfValue,
    pub object: RdfValue,
}

impl RdfStatement {
    pub fn new(subject: RdfValue, predicate: RdfValue, object: RdfValue) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }
}

pub fn write_rdf_value(value: &RdfValue) -> Vec<u8> {
    let mut out = Vec::new();
    write_rdf_value_to(&mut out, value);
    out
}

fn write_rdf_value_to(out: &mut Vec<u8>, value: &RdfValue) {
    match value {
        RdfValue::Iri(iri) => {
            out.push(URI_MARKER);
            out.extend(iri.as_bytes());
        }
        RdfValue::BNode(id) => {
            out.push(BNODE_MARKER);
            out.extend(id.as_bytes());
        }
        RdfValue::PlainLiteral(label) => {
            out.push(PLAIN_LITERAL_MARKER);
            out.extend(label.as_bytes());
        }
        RdfValue::LangLiteral { label, language } => {
            out.push(LANG_LITERAL_MARKER);
            out.extend(label.as_bytes());
            out.push(LANG_LITERAL_MARKER);
            out.extend(language.as_bytes());
        }
        RdfValue::DatatypeLiteral { label, datatype } => {
            out.push(DATATYPE_LITERAL_MARKER);
            out.extend(label.as_bytes());
            out.push(DATATYPE_LITERAL_MARKER);
            out.push(URI_MARKER);
            out.extend(datatype.as_bytes());
        }
    }
}

pub fn read_rdf_value(bytes: &[u8], delimiter: u8) -> Result<(RdfValue, usize), String> {
    let marker = *bytes
        .first()
        .ok_or_else(|| "Cannot read RDF value from empty bytes".to_string())?;
    match marker {
        URI_MARKER => read_simple(bytes, 1, delimiter).map(|(s, n)| (RdfValue::Iri(s), n)),
        BNODE_MARKER => read_simple(bytes, 1, delimiter).map(|(s, n)| (RdfValue::BNode(s), n)),
        PLAIN_LITERAL_MARKER => {
            read_simple(bytes, 1, delimiter).map(|(s, n)| (RdfValue::PlainLiteral(s), n))
        }
        LANG_LITERAL_MARKER => {
            let (label, used) = read_simple(bytes, 1, LANG_LITERAL_MARKER)?;
            let (language, used2) = read_simple(&bytes[used..], 0, delimiter)?;
            Ok((RdfValue::LangLiteral { label, language }, used + used2))
        }
        DATATYPE_LITERAL_MARKER => {
            let (label, used) = read_simple(bytes, 1, DATATYPE_LITERAL_MARKER)?;
            if bytes.get(used) != Some(&URI_MARKER) {
                return Err("Expected URI datatype marker".to_string());
            }
            let (datatype, used2) = read_simple(&bytes[used + 1..], 0, delimiter)?;
            Ok((
                RdfValue::DatatypeLiteral { label, datatype },
                used + 1 + used2,
            ))
        }
        _ => Err(format!("Invalid value type marker: {marker}")),
    }
}

fn read_simple(bytes: &[u8], start: usize, delimiter: u8) -> Result<(String, usize), String> {
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == delimiter)
        .map(|index| start + index)
        .unwrap_or(bytes.len());
    let value = String::from_utf8(bytes[start..end].to_vec()).map_err(|e| e.to_string())?;
    let used = if end < bytes.len() { end + 1 } else { end };
    Ok((value, used))
}

pub fn write_rdf_statement(statement: &RdfStatement, document_order: bool) -> Vec<u8> {
    let mut out = Vec::new();
    if document_order {
        write_rdf_value_to(&mut out, &statement.subject);
        out.push(FAMILY_DELIM);
        write_rdf_value_to(&mut out, &statement.predicate);
        out.push(FAMILY_DELIM);
        write_rdf_value_to(&mut out, &statement.object);
    } else {
        write_rdf_value_to(&mut out, &statement.predicate);
        out.push(INDEX_DELIM);
        write_rdf_value_to(&mut out, &statement.object);
        out.push(FAMILY_DELIM);
        write_rdf_value_to(&mut out, &statement.subject);
    }
    out
}

pub fn read_rdf_statement(bytes: &[u8], document_order: bool) -> Result<RdfStatement, String> {
    if document_order {
        let (subject, n1) = read_rdf_value(bytes, FAMILY_DELIM)?;
        let (predicate, n2) = read_rdf_value(&bytes[n1..], FAMILY_DELIM)?;
        let (object, _) = read_rdf_value(&bytes[n1 + n2..], FAMILY_DELIM)?;
        Ok(RdfStatement::new(subject, predicate, object))
    } else {
        let (predicate, n1) = read_rdf_value(bytes, INDEX_DELIM)?;
        let (object, n2) = read_rdf_value(&bytes[n1..], FAMILY_DELIM)?;
        let (subject, _) = read_rdf_value(&bytes[n1 + n2..], FAMILY_DELIM)?;
        Ok(RdfStatement::new(subject, predicate, object))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DateHashModShardValueGenerator {
    base_mod: i32,
}

impl Default for DateHashModShardValueGenerator {
    fn default() -> Self {
        Self { base_mod: 50 }
    }
}

impl DateHashModShardValueGenerator {
    pub fn new(base_mod: i32) -> Self {
        Self { base_mod }
    }

    pub fn generate_shard_value(&self, date_millis: i64, object: Option<&str>) -> String {
        let date = format_yyyymmdd(date_millis);
        match object {
            Some(object) => {
                let hash_mod = java_string_hash_code(object) % self.base_mod;
                format!("{}_{}", date, hash_mod.abs())
            }
            None => date,
        }
    }
}

fn java_string_hash_code(value: &str) -> i32 {
    let mut hash = 0i32;
    for unit in value.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(unit));
    }
    hash
}

fn format_yyyymmdd(epoch_millis: i64) -> String {
    let days = epoch_millis.div_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}{month:02}{day:02}")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + (month <= 2) as i64;
    (year, month, day)
}

pub fn split_shard_date(shard: &str, delimiter: &str) -> Option<(String, String)> {
    let index = shard.rfind(delimiter)?;
    Some((
        shard[..index].to_string(),
        shard[index + delimiter.len()..].to_string(),
    ))
}

pub fn retrieve_embed_key(key: &str) -> &str {
    key.split_once(INDEX_DELIM_STR)
        .map(|(prefix, _)| prefix)
        .unwrap_or(key)
}

#[cfg(test)]
#[path = "tests/partition_tests.rs"]
mod tests;
