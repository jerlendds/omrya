use crate::domain::{RDF_LANG_STRING, RyaIri, RyaType, XSD_ANY_URI, XSD_STRING};

pub mod datetime;
pub mod triple;

pub const DELIM_BYTE: u8 = 0;
pub const TYPE_DELIM_BYTE: u8 = 1;
pub const URI_MARKER: u8 = 2;
pub const PLAIN_LITERAL_MARKER: u8 = 3;
pub const DT_LITERAL_MARKER: u8 = 8;
pub const LANGUAGE_DELIMITER: char = '@';
pub const UNDETERMINED_LANGUAGE: &str = "und";

pub fn serialize(rya_type: &RyaType) -> Result<Vec<u8>, String> {
    let (data, type_suffix) = serialize_type(rya_type)?;
    let mut bytes = data;
    bytes.extend(type_suffix);
    Ok(bytes)
}

pub fn serialize_type(rya_type: &RyaType) -> Result<(Vec<u8>, Vec<u8>), String> {
    let data_type = rya_type
        .data_type()
        .ok_or_else(|| "RyaType is missing a datatype".to_string())?;
    let data = append_language(rya_type.data(), rya_type.language(), data_type);
    let data_bytes = encode_string(&data).into_bytes();

    match data_type {
        XSD_STRING => Ok((data_bytes, vec![TYPE_DELIM_BYTE, PLAIN_LITERAL_MARKER])),
        XSD_ANY_URI => Ok((data_bytes, vec![TYPE_DELIM_BYTE, URI_MARKER])),
        _ => {
            let mut suffix = Vec::with_capacity(data_type.len() + 2);
            suffix.push(TYPE_DELIM_BYTE);
            suffix.extend(data_type.as_bytes());
            suffix.push(TYPE_DELIM_BYTE);
            suffix.push(DT_LITERAL_MARKER);
            Ok((data_bytes, suffix))
        }
    }
}

pub fn deserialize(bytes: &[u8]) -> Result<RyaType, String> {
    let marker = *bytes
        .last()
        .ok_or_else(|| "Cannot deserialize empty byte slice".to_string())?;
    match marker {
        PLAIN_LITERAL_MARKER => deserialize_builtin(bytes, XSD_STRING, false),
        URI_MARKER => deserialize_builtin(bytes, XSD_ANY_URI, true),
        DT_LITERAL_MARKER => deserialize_custom(bytes),
        _ => Err(format!("Unknown Rya type marker byte: {marker}")),
    }
}

fn deserialize_builtin(bytes: &[u8], data_type: &str, iri: bool) -> Result<RyaType, String> {
    ensure_builtin_suffix(bytes)?;
    let data = decode_string(std::str::from_utf8(&bytes[..bytes.len() - 2]).map_err(to_string)?);
    if iri {
        return Ok(RyaIri::new(data)?.into_type());
    }
    Ok(rya_type_from_serialized_parts(data_type.to_string(), data))
}

fn deserialize_custom(bytes: &[u8]) -> Result<RyaType, String> {
    if bytes.len() < 4
        || bytes[bytes.len() - 1] != DT_LITERAL_MARKER
        || bytes[bytes.len() - 2] != TYPE_DELIM_BYTE
    {
        return Err("Bytes not deserializable as a custom datatype".to_string());
    }

    let data_type_start = bytes
        .iter()
        .position(|byte| *byte == TYPE_DELIM_BYTE)
        .ok_or_else(|| "Not a datatype literal".to_string())?;
    if data_type_start < 1 {
        return Err("Not a datatype literal".to_string());
    }

    let data = decode_string(std::str::from_utf8(&bytes[..data_type_start]).map_err(to_string)?);
    let data_type =
        std::str::from_utf8(&bytes[data_type_start + 1..bytes.len() - 2]).map_err(to_string)?;
    Ok(rya_type_from_serialized_parts(data_type.to_string(), data))
}

fn ensure_builtin_suffix(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 2 || bytes[bytes.len() - 2] != TYPE_DELIM_BYTE {
        Err("Bytes not deserializable".to_string())
    } else {
        Ok(())
    }
}

fn append_language(data: &str, language: Option<&str>, data_type: &str) -> String {
    match validate_language(language, data_type) {
        Some(language) => format!("{data}{LANGUAGE_DELIMITER}{language}"),
        None => data.to_string(),
    }
}

fn validate_language(language: Option<&str>, data_type: &str) -> Option<String> {
    if data_type != RDF_LANG_STRING {
        return None;
    }
    match language {
        Some(language) if is_valid_language_tag(language) => Some(language.to_string()),
        _ => Some(UNDETERMINED_LANGUAGE.to_string()),
    }
}

fn rya_type_from_serialized_parts(data_type: String, mut data: String) -> RyaType {
    if data_type == RDF_LANG_STRING {
        let (parsed_data, language) = parse_language_data(&data);
        data = parsed_data;
        RyaType::from_parts(Some(data_type), data, Some(language))
    } else {
        RyaType::from_parts(Some(data_type), data, None)
    }
}

fn parse_language_data(data: &str) -> (String, String) {
    match data.rsplit_once(LANGUAGE_DELIMITER) {
        Some((parsed_data, language)) if is_valid_language_tag(language) => {
            (parsed_data.to_string(), language.to_string())
        }
        Some((parsed_data, _)) => (parsed_data.to_string(), UNDETERMINED_LANGUAGE.to_string()),
        None => (data.to_string(), UNDETERMINED_LANGUAGE.to_string()),
    }
}

fn is_valid_language_tag(language: &str) -> bool {
    !language.is_empty()
        && language
            .split('-')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric()))
}

fn encode_string(data: &str) -> String {
    data.to_string()
}

fn decode_string(data: &str) -> String {
    data.to_string()
}

fn to_string(error: impl std::error::Error) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "../tests/resolver_mod_tests.rs"]
mod tests;
