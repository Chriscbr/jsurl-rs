//! This crate is a Rust implementation of the [jsurl](https://github.com/Sage/jsurl)
//! serialization format. It is a more compact and human-readable alternative to plain URL encoding
//! for including JSON in URLs.
//!
//! # Example
//!
//! ```rust
//! use jsurl::{deserialize, serialize};
//! use serde_json::json;
//!
//! let obj = json!({
//!     "name": "John Doe",
//!     "age": 42,
//!     "children": ["Mary", "Bill"]
//! });
//!
//! let serialized = serialize(&obj);
//! assert_eq!(serialized, "~(name~'John*20Doe~age~42~children~(~'Mary~'Bill))");
//!
//! let deserialized = deserialize("~(name~'John*20Doe~age~42~children~(~'Mary~'Bill))").unwrap();
//! assert_eq!(deserialized, obj);
//! ```

pub fn serialize(obj: &serde_json::Value) -> String {
    let mut result = String::new();
    serialize_helper(obj, &mut result);
    result
}

pub fn serialize_helper(obj: &serde_json::Value, output: &mut String) {
    match obj {
        serde_json::Value::Null => {
            output.push_str("~null");
        }
        serde_json::Value::Bool(b) => {
            output.push('~');
            output.push_str(if *b { "true" } else { "false" });
        }
        serde_json::Value::Number(n) => {
            if let Some(n) = n.as_i64() {
                output.push('~');
                output.push_str(&n.to_string());
            } else if let Some(n) = n.as_f64() {
                if n.is_finite() {
                    output.push('~');
                    output.push_str(&n.to_string());
                } else {
                    // https://github.com/Sage/jsurl/blob/b1e244d145bb440f776d8fec673cc743c42c5cbc/lib/jsurl.js#L42
                    output.push_str("~null");
                }
            } else {
                panic!("Unexpected number type")
            }
        }
        serde_json::Value::String(s) => {
            output.push_str("~'");
            encode_string(s, output);
        }
        serde_json::Value::Array(a) => {
            output.push_str("~(");
            if a.is_empty() {
                output.push('~');
            } else {
                for v in a.iter() {
                    serialize_helper(v, output);
                }
            }
            output.push(')');
        }
        serde_json::Value::Object(o) => {
            output.push_str("~(");
            for (i, (k, v)) in o.iter().enumerate() {
                if i > 0 {
                    output.push('~');
                }
                encode_string(k, output);
                serialize_helper(v, output);
            }
            output.push(')');
        }
    }
}

fn encode_string(s: &str, output: &mut String) {
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            output.push(ch);
        } else if ch == '$' {
            output.push('!');
        } else {
            let code = ch as u32;
            if code < 0x100 {
                output.push_str(&format!("*{:02x}", code));
            } else {
                output.push_str(&format!("**{:04x}", code));
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeserializeError;

pub fn deserialize(s: &str) -> Result<serde_json::Value, DeserializeError> {
    let mut chars = s.chars();
    let result = parse_one(&mut chars)?;
    if chars.next().is_some() {
        return Err(DeserializeError);
    }
    Ok(result)
}

fn hex_digit_to_value(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        'a'..='f' => Some(10 + c as u32 - 'a' as u32),
        'A'..='F' => Some(10 + c as u32 - 'A' as u32),
        _ => None,
    }
}

fn hex2_to_unicode(a: char, b: char) -> Option<char> {
    let high = hex_digit_to_value(a)?;
    let low = hex_digit_to_value(b)?;
    std::char::from_u32((high << 4) | low)
}

fn hex4_to_unicode(a: char, b: char, c: char, d: char) -> Option<char> {
    let highest = hex_digit_to_value(a)?;
    let high = hex_digit_to_value(b)?;
    let low = hex_digit_to_value(c)?;
    let lowest = hex_digit_to_value(d)?;
    std::char::from_u32((highest << 12) | (high << 8) | (low << 4) | lowest)
}

fn decode(chars: &mut std::str::Chars) -> Result<String, DeserializeError> {
    let mut result = String::new();
    loop {
        let c = peek(chars);
        match c {
            Some('~') | Some(')') => {
                return Ok(result);
            }
            Some('*') => {
                chars.next();
                match chars.next() {
                    // case: character with unicode value > 0xff
                    Some('*') => {
                        let x1 = chars.next().ok_or(DeserializeError)?;
                        let x2 = chars.next().ok_or(DeserializeError)?;
                        let x3 = chars.next().ok_or(DeserializeError)?;
                        let x4 = chars.next().ok_or(DeserializeError)?;

                        result.push(hex4_to_unicode(x1, x2, x3, x4).ok_or(DeserializeError)?);
                    }
                    // case: character with unicode value <= 0xff
                    Some(c) => {
                        let x1 = c;
                        let x2 = chars.next().ok_or(DeserializeError)?;

                        result.push(hex2_to_unicode(x1, x2).ok_or(DeserializeError)?);
                    }
                    None => {
                        return Err(DeserializeError);
                    }
                }
            }
            Some('!') => {
                result.push('$');
                chars.next();
            }
            Some(c) => {
                result.push(c);
                chars.next();
            }
            None => return Ok(result),
        }
    }
}

fn peek(chars: &std::str::Chars) -> Option<char> {
    let mut iter = chars.clone();
    iter.next()
}

fn peekn(chars: &std::str::Chars, n: usize) -> Option<char> {
    let mut iter = chars.clone();
    for _ in 0..n {
        iter.next();
    }
    iter.next()
}

fn eat(chars: &mut std::str::Chars, expected: char) -> Result<(), DeserializeError> {
    match chars.next() {
        Some(c) if c == expected => Ok(()),
        _ => Err(DeserializeError),
    }
}

fn parse_array(chars: &mut std::str::Chars) -> Result<serde_json::Value, DeserializeError> {
    // handle case where empty array is represented as "~(~)"
    if let Some(')') = peekn(chars, 1) {
        eat(chars, '~')?;
        eat(chars, ')')?;
        return Ok(serde_json::Value::Array(Vec::new()));
    }
    let mut result = Vec::new();
    loop {
        if let Some(')') = peek(chars) {
            chars.next();
            return Ok(serde_json::Value::Array(result));
        }
        result.push(parse_one(chars)?);
    }
}

fn parse_object(chars: &mut std::str::Chars) -> Result<serde_json::Value, DeserializeError> {
    let mut map = serde_json::Map::new();
    while let Some(c) = peek(chars) {
        if c == '~' || c == ')' {
            chars.next();
        }
        if c == ')' {
            break;
        }
        let key = decode(chars)?;
        let value = parse_one(chars)?;
        map.insert(key, value);
        if peek(chars).map_or(false, |c| c != '~' && c != ')') {
            return Err(DeserializeError);
        }
    }
    Ok(serde_json::Value::Object(map))
}

fn parse_one(chars: &mut std::str::Chars) -> Result<serde_json::Value, DeserializeError> {
    eat(chars, '~')?;
    match chars.next() {
        Some('(') => {
            if let Some('~') = peek(chars) {
                parse_array(chars)
            } else {
                parse_object(chars)
            }
        }
        Some('\'') => Ok(serde_json::Value::String(decode(chars)?)),
        Some(c) => {
            let mut result = String::new();
            result.push(c);
            loop {
                match peek(chars) {
                    Some(')') | Some('~') | None => {
                        match result.as_str() {
                            "null" => return Ok(serde_json::Value::Null),
                            "true" => return Ok(serde_json::Value::Bool(true)),
                            "false" => return Ok(serde_json::Value::Bool(false)),
                            _ => {}
                        }
                        match result.chars().next() {
                            Some(c) if c == '-' || c.is_ascii_digit() => {
                                return Ok(serde_json::Value::Number(
                                    result.parse().map_err(|_| DeserializeError)?,
                                ));
                            }
                            _ => return Err(DeserializeError),
                        }
                    }
                    Some(c) => {
                        result.push(c);
                        chars.next();
                    }
                }
            }
        }
        None => Err(DeserializeError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[macro_export]
    macro_rules! assert_serialize_eq {
        ($json_str:expr, $expected:expr $(,)?) => {
            let obj: serde_json::Value = serde_json::from_str($json_str).expect("invalid JSON");
            let serialized = serialize(&obj);
            assert_eq!(serialized, $expected);
        };
    }

    #[macro_export]
    macro_rules! assert_deserialize_eq {
        ($expected:expr, $jsurl_str:expr $(,)?) => {
            let obj: serde_json::Value = serde_json::from_str($expected).expect("invalid JSON");
            let deserialized = deserialize($jsurl_str).expect("invalid jsurl");
            assert_eq!(deserialized, obj);
        };
    }

    #[test]
    fn serialize_basic_values() {
        assert_serialize_eq!("null", "~null");
        assert_serialize_eq!("true", "~true");
        assert_serialize_eq!("false", "~false");
        assert_serialize_eq!("0", "~0");
        assert_serialize_eq!("1", "~1");
        assert_serialize_eq!("-1.5", "~-1.5");

        // note: Infinity, -Infinity, and NaN are not valid JSON numbers
        // so they cannot be represented with serde_json::Value
    }

    #[test]
    fn serialize_strings() {
        let s1 = serde_json::Value::String("hello world\u{203c}".to_string());
        assert_eq!(serialize(&s1), "~'hello*20world**203c");

        let s2 = serde_json::Value::String(" !\"#$%&'()*+,-./09:;<=>?@AZ[\\]^_`az{|}~".to_string());
        assert_eq!(serialize(&s2), "~'*20*21*22*23!*25*26*27*28*29*2a*2b*2c-.*2f09*3a*3b*3c*3d*3e*3f*40AZ*5b*5c*5d*5e_*60az*7b*7c*7d*7e");

        let s3 = serde_json::Value::String("".to_string());
        assert_eq!(serialize(&s3), "~'");
    }

    #[test]
    fn serialize_arrays() {
        assert_serialize_eq!("[]", "~(~)");
        assert_serialize_eq!("[1,2,3]", "~(~1~2~3)");
        assert_serialize_eq!(
            r#"[null, false, 0, "hello world\u203c"]"#,
            "~(~null~false~0~'hello*20world**203c)"
        );
    }

    #[test]
    fn serialize_objects() {
        assert_serialize_eq!("{}", "~()");
        assert_serialize_eq!(
            r#"{"c":null,"d":false,"e":0,"f":"hello world\u203c"}"#,
            "~(c~null~d~false~e~0~f~'hello*20world**203c)"
        );
        assert_serialize_eq!(
            r#"{"a": [[1, 2],[], {}],"b": [],"c": {"d": "hello","e": {},"f": []}}"#,
            "~(a~(~(~1~2)~(~)~())~b~(~)~c~(d~'hello~e~()~f~(~)))",
        );
    }

    #[test]
    fn serialize_example() {
        assert_serialize_eq!(
            r#"{"name":"John Doe","age":42,"children":["Mary","Bill"]}"#,
            "~(name~'John*20Doe~age~42~children~(~'Mary~'Bill))",
        );
    }

    #[test]
    fn deserialize_basic_values() {
        assert_deserialize_eq!("null", "~null");
        assert_deserialize_eq!("true", "~true");
        assert_deserialize_eq!("false", "~false");
        assert_deserialize_eq!("0", "~0");
        assert_deserialize_eq!("1", "~1");
        assert_deserialize_eq!("-1.5", "~-1.5");
    }

    #[test]
    fn deserialize_strings() {
        let s1 = serde_json::Value::String("hello world\u{203c}".to_string());
        assert_eq!(deserialize("~'hello*20world**203c").unwrap(), s1);

        let s2 = serde_json::Value::String(" !\"#$%&'()*+,-./09:;<=>?@AZ[\\]^_`az{|}~".to_string());
        assert_eq!(deserialize("~'*20*21*22*23!*25*26*27*28*29*2a*2b*2c-.*2f09*3a*3b*3c*3d*3e*3f*40AZ*5b*5c*5d*5e_*60az*7b*7c*7d*7e").unwrap(), s2);
    }

    #[test]
    fn deserialize_arrays() {
        assert_deserialize_eq!("[]", "~(~)");
        assert_deserialize_eq!("[1,2,3]", "~(~1~2~3)");
        assert_deserialize_eq!(
            r#"[null, false, 0, "hello world\u203c"]"#,
            "~(~null~false~0~'hello*20world**203c)"
        );
    }

    #[test]
    fn deserialize_objects() {
        assert_deserialize_eq!("{}", "~()");
        assert_deserialize_eq!(
            r#"{"c":null,"d":false,"e":0,"f":"hello world\u203c"}"#,
            "~(c~null~d~false~e~0~f~'hello*20world**203c)"
        );
        assert_deserialize_eq!(
            r#"{"a": [[1, 2],[], {"a1": 3}],"b": [],"c": {"d": "hello","e": {},"f": []}}"#,
            "~(a~(~(~1~2)~(~)~(a1~3))~b~(~)~c~(d~'hello~e~()~f~(~)))",
        );
    }

    #[test]
    fn deserialize_example() {
        assert_deserialize_eq!(
            r#"{"name":"John Doe","age":42,"children":["Mary","Bill"]}"#,
            "~(name~'John*20Doe~age~42~children~(~'Mary~'Bill))",
        );
    }

    #[test]
    fn deserialize_error() {
        assert_eq!(deserialize("").unwrap_err(), DeserializeError);
        assert_eq!(deserialize("hello world").unwrap_err(), DeserializeError);
        assert_eq!(deserialize("~").unwrap_err(), DeserializeError);
        assert_eq!(deserialize("~cool").unwrap_err(), DeserializeError);
    }
}
