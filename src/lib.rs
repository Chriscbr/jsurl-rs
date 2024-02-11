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
pub enum DeserializeError {
    InvalidValue(String),
    InvalidEscapeSequence(String),
    UnexpectedCharacter(char, Option<char>), // actual, expected
    UnexpectedEnd,
}

pub fn deserialize(s: &str) -> Result<serde_json::Value, DeserializeError> {
    let mut chars = s.chars();
    let result = parse_one(&mut chars)?;
    if chars.next().is_some() {
        return Err(DeserializeError::UnexpectedCharacter(
            chars.next().unwrap(),
            None,
        ));
    }
    Ok(result)
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
                        let hex1 = chars.next().ok_or(DeserializeError::UnexpectedEnd)?;
                        let hex2 = chars.next().ok_or(DeserializeError::UnexpectedEnd)?;
                        let hex3 = chars.next().ok_or(DeserializeError::UnexpectedEnd)?;
                        let hex4 = chars.next().ok_or(DeserializeError::UnexpectedEnd)?;

                        // check that both hex1 and hex2 are valid hex characters
                        if !hex1.is_ascii_hexdigit() {
                            return Err(DeserializeError::UnexpectedCharacter(hex1, None));
                        }
                        if !hex2.is_ascii_hexdigit() {
                            return Err(DeserializeError::UnexpectedCharacter(hex2, None));
                        }
                        if !hex3.is_ascii_hexdigit() {
                            return Err(DeserializeError::UnexpectedCharacter(hex3, None));
                        }
                        if !hex4.is_ascii_hexdigit() {
                            return Err(DeserializeError::UnexpectedCharacter(hex4, None));
                        }

                        // to avoid a heap allocation, we use a stack-allocated buffer
                        let mut buf: [u8; 16] = [0; 16];
                        let size_c1 = hex1.encode_utf8(&mut buf).len();
                        let size_c2 = hex2.encode_utf8(&mut buf[size_c1..]).len();
                        let size_c3 = hex3.encode_utf8(&mut buf[size_c1 + size_c2..]).len();
                        let size_c4 = hex4
                            .encode_utf8(&mut buf[size_c1 + size_c2 + size_c3..])
                            .len();
                        let chars_str =
                            std::str::from_utf8(&buf[..size_c1 + size_c2 + size_c3 + size_c4])
                                .unwrap();

                        let code = u32::from_str_radix(chars_str, 16).map_err(|_| {
                            DeserializeError::InvalidEscapeSequence(chars_str.to_string())
                        })?;
                        result.push(std::char::from_u32(code).ok_or(
                            DeserializeError::InvalidEscapeSequence(chars_str.to_string()),
                        )?);
                    }
                    // case: character with unicode value <= 0xff
                    Some(c) => {
                        let hex1 = c;
                        let hex2 = chars.next().ok_or(DeserializeError::UnexpectedEnd)?;

                        // check that both hex1 and hex2 are valid hex characters
                        if !hex1.is_ascii_hexdigit() {
                            return Err(DeserializeError::UnexpectedCharacter(hex1, None));
                        }
                        if !hex2.is_ascii_hexdigit() {
                            return Err(DeserializeError::UnexpectedCharacter(hex2, None));
                        }

                        // to avoid a heap allocation, we use a stack-allocated buffer
                        let mut buf: [u8; 8] = [0; 8];
                        let size_c1 = hex1.encode_utf8(&mut buf).len();
                        let size_c2 = hex2.encode_utf8(&mut buf[size_c1..]).len();
                        let chars_str = std::str::from_utf8(&buf[..size_c1 + size_c2]).unwrap();

                        let code = u32::from_str_radix(chars_str, 16).map_err(|_| {
                            DeserializeError::InvalidEscapeSequence(chars_str.to_string())
                        })?;
                        result.push(std::char::from_u32(code).ok_or(
                            DeserializeError::InvalidEscapeSequence(chars_str.to_string()),
                        )?);
                    }
                    None => {
                        return Err(DeserializeError::UnexpectedEnd);
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

fn eat(chars: &mut std::str::Chars, expected: char) -> Result<(), DeserializeError> {
    match chars.next() {
        Some(c) if c == expected => Ok(()),
        Some(c) => Err(DeserializeError::UnexpectedCharacter(c, Some(expected))),
        None => Err(DeserializeError::UnexpectedEnd),
    }
}

fn parse_one(chars: &mut std::str::Chars) -> Result<serde_json::Value, DeserializeError> {
    eat(chars, '~')?;
    let c = peek(chars);
    match c {
        Some('(') => {
            chars.next();
            let c = peek(chars);
            if c == Some('~') {
                // parse as an array
                let c = peekn(chars, 1);
                if c == Some(')') {
                    chars.next();
                    chars.next();
                    return Ok(serde_json::Value::Array(vec![]));
                }
                let mut result = Vec::new();
                loop {
                    let c = peek(chars);
                    if c == Some(')') {
                        chars.next();
                        return Ok(serde_json::Value::Array(result));
                    }
                    result.push(parse_one(chars)?);
                }
            } else {
                // parse as an object
                let mut map = serde_json::Map::new();
                if c == Some(')') {
                    chars.next();
                    return Ok(serde_json::Value::Object(map));
                }
                loop {
                    let key = decode(chars)?;
                    let value = parse_one(chars)?;
                    map.insert(key, value);
                    let c = peek(chars);
                    if c == Some('~') {
                        chars.next();
                    } else if c == Some(')') {
                        chars.next();
                        return Ok(serde_json::Value::Object(map));
                    } else {
                        return Err(DeserializeError::UnexpectedCharacter(c.unwrap(), None));
                    }
                }
            }
        }
        Some('\'') => {
            chars.next();
            Ok(serde_json::Value::String(decode(chars)?))
        }
        Some(c) => {
            // keep consuming characters until we reach ), ~, or the end of our input
            let mut result = String::new();
            result.push(c);
            chars.next();
            loop {
                let c = peek(chars);
                match c {
                    Some(')') | Some('~') | None => {
                        match result.as_str() {
                            "null" => return Ok(serde_json::Value::Null),
                            "true" => return Ok(serde_json::Value::Bool(true)),
                            "false" => return Ok(serde_json::Value::Bool(false)),
                            _ => {}
                        }
                        let c = result.chars().next().unwrap();
                        if c == '-' || c.is_ascii_digit() {
                            return Ok(serde_json::Value::Number(result.parse().map_err(
                                |_| DeserializeError::InvalidValue(result.to_string()),
                            )?));
                        }
                        return Err(DeserializeError::InvalidValue(result));
                    }
                    Some(c) => {
                        result.push(c);
                        chars.next();
                    }
                }
            }
        }
        None => Err(DeserializeError::UnexpectedEnd),
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
            r#"{"a": [[1, 2],[], {}],"b": [],"c": {"d": "hello","e": {},"f": []}}"#,
            "~(a~(~(~1~2)~(~)~())~b~(~)~c~(d~'hello~e~()~f~(~)))",
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
        assert_eq!(
            deserialize("~").unwrap_err(),
            DeserializeError::UnexpectedEnd
        );

        assert_eq!(
            deserialize("~cool").unwrap_err(),
            DeserializeError::InvalidValue("cool".to_string())
        );
    }
}
