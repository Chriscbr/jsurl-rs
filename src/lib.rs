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
        // If the character is alphanumeric, a dot, an underscore, or a hyphen, it doesn't need to be encoded.
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
    fn serialize_string_values() {
        let s1 = serde_json::Value::String("hello world\u{203c}".to_string());
        assert_eq!(serialize(&s1), "~'hello*20world**203c");

        let s2 = serde_json::Value::String(" !\"#$%&'()*+,-./09:;<=>?@AZ[\\]^_`az{|}~".to_string());
        assert_eq!(serialize(&s2), "~'*20*21*22*23!*25*26*27*28*29*2a*2b*2c-.*2f09*3a*3b*3c*3d*3e*3f*40AZ*5b*5c*5d*5e_*60az*7b*7c*7d*7e");
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
}
