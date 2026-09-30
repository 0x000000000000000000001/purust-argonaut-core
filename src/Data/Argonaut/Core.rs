use std::rc::Rc;

// Json values are the boxed runtime representation: scalars use the native
// Value variants, arrays the runtime array storage and objects the native
// Foreign.Object carrier, exactly like the JavaScript driver's values.
pub fn purust_json_quote(value: &str) -> String {
    let units = purust_core::purust_string_to_utf16(value);
    let mut output = String::from("\"");
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        match unit {
            0x22 => output.push_str("\\\""),
            0x5c => output.push_str("\\\\"),
            8 => output.push_str("\\b"),
            9 => output.push_str("\\t"),
            10 => output.push_str("\\n"),
            12 => output.push_str("\\f"),
            13 => output.push_str("\\r"),
            0..=0x1f => output.push_str(&format!("\\u{:04x}", unit)),
            0xd800..=0xdbff
                if units
                    .get(i + 1)
                    .is_some_and(|v| (0xdc00..=0xdfff).contains(v)) =>
            {
                output.push(purust_core::purust_char_from_code_unit(unit));
                i += 1;
                output.push(purust_core::purust_char_from_code_unit(units[i]));
            }
            0xd800..=0xdfff => output.push_str(&format!("\\u{:04x}", unit)),
            _ => output.push(purust_core::purust_char_from_code_unit(unit)),
        }
        i += 1;
    }
    output.push('"');
    output
}

fn purust_json_number(value: f64) -> String {
    if !value.is_finite() {
        "null".into()
    } else {
        ryu_js::Buffer::new().format_finite(value).to_owned()
    }
}

fn purust_json_object_value(value: &crate::UnknownType) -> Option<Rc<Purs_Foreign_Object::Object>> {
    match value.resolve() {
        crate::Value::Class(native) => native.downcast_ref::<Rc<Purs_Foreign_Object::Object>>().cloned(),
        _ => None,
    }
}

// Driver-facing accessors: the benchmark harness reads the corpus through the
// same boxed representation the parser produces.
pub fn purust_json_object_get(value: &crate::UnknownType, key: &str) -> Option<crate::UnknownType> {
    purust_json_object_value(value).and_then(|object| object.get(key))
}

pub fn purust_json_array_items(value: &crate::UnknownType) -> Option<Vec<crate::UnknownType>> {
    match value.resolve() {
        crate::Value::Array(values) => Some(values.iter().cloned().collect()),
        _ => None,
    }
}

fn purust_json_fields(object: &Purs_Foreign_Object::Object) -> String {
    let parts: Vec<_> = object
        .entries()
        .into_iter()
        .map(|(key, value)| format!("{}:{}", purust_json_quote(&key), purust_json_value(&value)))
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn purust_json_value(value: &crate::UnknownType) -> String {
    match value.resolve() {
        crate::Value::Null => "null".into(),
        crate::Value::Int(n) => n.to_string(),
        crate::Value::Number(n) => purust_json_number(*n),
        crate::Value::Bool(b) => b.to_string(),
        crate::Value::String(s) => purust_json_quote(s),
        crate::Value::Char(c) => purust_json_quote(&c.to_string()),
        crate::Value::IntArray(values) => format!(
            "[{}]",
            values.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
        ),
        crate::Value::Array(values) => format!(
            "[{}]",
            values.iter().map(purust_json_value).collect::<Vec<_>>().join(",")
        ),
        crate::Value::Class(native) => match native.downcast_ref::<Rc<Purs_Foreign_Object::Object>>() {
            Some(object) => purust_json_fields(object),
            None => panic!("Data.Argonaut.Core: unsupported opaque Json value"),
        },
        _ => panic!("Data.Argonaut.Core: value has no JSON representation"),
    }
}

pub fn Data_Argonaut_Core_fromBoolean(value: bool) -> crate::UnknownType {
    crate::Value::Bool(value)
}

pub fn Data_Argonaut_Core_fromNumber(value: f64) -> crate::UnknownType {
    crate::Value::Number(value)
}

pub fn Data_Argonaut_Core_fromString(value: String) -> crate::UnknownType {
    crate::Value::String(value)
}

pub fn Data_Argonaut_Core_fromArray(value: crate::UnknownType) -> crate::UnknownType {
    match value.resolve() {
        crate::Value::Array(_) | crate::Value::IntArray(_) => value.clone(),
        _ => panic!("Data.Argonaut.Core: fromArray expects an array"),
    }
}

pub fn Data_Argonaut_Core_fromObject(value: Rc<Purs_Foreign_Object::Object>) -> crate::UnknownType {
    crate::Value::Class(Rc::new(value))
}

pub fn Data_Argonaut_Core_jsonNull() -> crate::UnknownType {
    crate::Value::Null
}

pub fn Data_Argonaut_Core_stringify(value: crate::UnknownType) -> String {
    purust_json_value(&value)
}

pub fn Data_Argonaut_Core_stringifyWithIndent(spaces: i64, value: crate::UnknownType) -> String {
    let compact = purust_json_value(&value);
    let gap = " ".repeat(spaces.clamp(0, 10) as usize);
    if gap.is_empty() {
        return compact;
    }
    let mut result = String::new();
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    let chars: Vec<_> = compact.chars().collect();
    for (index, &ch) in chars.iter().enumerate() {
        if quoted {
            result.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => {
                quoted = true;
                result.push(ch);
            }
            '[' | '{' => {
                result.push(ch);
                depth += 1;
                if chars.get(index + 1) != Some(&if ch == '[' { ']' } else { '}' }) {
                    result.push('\n');
                    result.push_str(&gap.repeat(depth));
                }
            }
            ']' | '}' => {
                depth -= 1;
                if index > 0 && chars[index - 1] != if ch == ']' { '[' } else { '{' } {
                    result.push('\n');
                    result.push_str(&gap.repeat(depth));
                }
                result.push(ch);
            }
            ',' => {
                result.push_str(",\n");
                result.push_str(&gap.repeat(depth));
            }
            ':' => result.push_str(": "),
            _ => result.push(ch),
        }
    }
    result
}

fn purust_json_object(value: &crate::UnknownType) -> Rc<Purs_Foreign_Object::Object> {
    purust_json_object_value(value).unwrap_or_else(|| {
        panic!("Data.Argonaut.Core: expected a Json object")
    })
}

pub fn Data_Argonaut_Core__caseJsonNull(
    default: crate::UnknownType,
    on_null: purust_core::Func1<(), crate::UnknownType>,
    value: crate::UnknownType,
) -> crate::UnknownType {
    match value.resolve() {
        crate::Value::Null => on_null(()),
        _ => default,
    }
}

pub fn Data_Argonaut_Core__caseJsonBoolean(
    default: crate::UnknownType,
    on_boolean: purust_core::Func1<bool, crate::UnknownType>,
    value: crate::UnknownType,
) -> crate::UnknownType {
    match value.resolve() {
        crate::Value::Bool(flag) => on_boolean(*flag),
        _ => default,
    }
}

pub fn Data_Argonaut_Core__caseJsonNumber(
    default: crate::UnknownType,
    on_number: purust_core::Func1<f64, crate::UnknownType>,
    value: crate::UnknownType,
) -> crate::UnknownType {
    match value.resolve() {
        crate::Value::Int(number) => on_number(*number as f64),
        crate::Value::Number(number) => on_number(*number),
        _ => default,
    }
}

pub fn Data_Argonaut_Core__caseJsonString(
    default: crate::UnknownType,
    on_string: purust_core::Func1<String, crate::UnknownType>,
    value: crate::UnknownType,
) -> crate::UnknownType {
    match value.resolve() {
        crate::Value::String(text) => on_string(text.clone()),
        crate::Value::Char(character) => on_string(character.to_string()),
        _ => default,
    }
}

pub fn Data_Argonaut_Core__caseJsonArray(
    default: crate::UnknownType,
    on_array: purust_core::Func1<crate::UnknownType, crate::UnknownType>,
    value: crate::UnknownType,
) -> crate::UnknownType {
    match value.resolve() {
        crate::Value::Array(_) | crate::Value::IntArray(_) => on_array(value.clone()),
        _ => default,
    }
}

pub fn Data_Argonaut_Core__caseJsonObject(
    default: crate::UnknownType,
    on_object: purust_core::Func1<Rc<Purs_Foreign_Object::Object>, crate::UnknownType>,
    value: crate::UnknownType,
) -> crate::UnknownType {
    match purust_json_object_value(&value) {
        Some(object) => on_object(object),
        None => default,
    }
}

pub fn Data_Argonaut_Core_caseJsonImpl(
    on_null: purust_core::Func1<(), crate::UnknownType>,
    on_boolean: purust_core::Func1<bool, crate::UnknownType>,
    on_number: purust_core::Func1<f64, crate::UnknownType>,
    on_string: purust_core::Func1<String, crate::UnknownType>,
    on_array: purust_core::Func1<crate::UnknownType, crate::UnknownType>,
    on_object: purust_core::Func1<Rc<Purs_Foreign_Object::Object>, crate::UnknownType>,
    value: crate::UnknownType,
) -> crate::UnknownType {
    match value.resolve() {
        crate::Value::Null => on_null(()),
        crate::Value::Bool(flag) => on_boolean(*flag),
        crate::Value::Int(number) => on_number(*number as f64),
        crate::Value::Number(number) => on_number(*number),
        crate::Value::String(text) => on_string(text.clone()),
        crate::Value::Char(character) => on_string(character.to_string()),
        crate::Value::Array(_) | crate::Value::IntArray(_) => on_array(value.clone()),
        crate::Value::Class(_) => on_object(purust_json_object(&value)),
        _ => panic!("Data.Argonaut.Core: caseJson on a non-Json value"),
    }
}

enum JsonRank {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

fn purust_json_rank(value: &crate::UnknownType) -> JsonRank {
    match value.resolve() {
        crate::Value::Null => JsonRank::Null,
        crate::Value::Bool(_) => JsonRank::Boolean,
        crate::Value::Int(_) | crate::Value::Number(_) => JsonRank::Number,
        crate::Value::String(_) | crate::Value::Char(_) => JsonRank::String,
        crate::Value::Array(_) | crate::Value::IntArray(_) => JsonRank::Array,
        crate::Value::Class(_) => JsonRank::Object,
        _ => panic!("Data.Argonaut.Core: compare on a non-Json value"),
    }
}

fn rank_order(rank: &JsonRank) -> u8 {
    match rank {
        JsonRank::Null => 0,
        JsonRank::Boolean => 1,
        JsonRank::Number => 2,
        JsonRank::String => 3,
        JsonRank::Array => 4,
        JsonRank::Object => 5,
    }
}

fn purust_json_string_units(value: &crate::UnknownType) -> Vec<u16> {
    match value.resolve() {
        crate::Value::String(text) => purust_core::purust_string_to_utf16(text),
        crate::Value::Char(character) => character.to_string().encode_utf16().collect(),
        _ => unreachable!(),
    }
}

fn purust_json_array_values(value: &crate::UnknownType) -> Vec<crate::UnknownType> {
    match value.resolve() {
        crate::Value::Array(values) => values.iter().cloned().collect(),
        crate::Value::IntArray(values) => values
            .iter()
            .map(|n| crate::Value::Number(*n as f64))
            .collect(),
        _ => unreachable!(),
    }
}

enum PurustJsonOrdering {
    Less,
    Equal,
    Greater,
}

fn purust_json_compare_values(
    left: &crate::UnknownType,
    right: &crate::UnknownType,
) -> PurustJsonOrdering {
    let left_rank = rank_order(&purust_json_rank(left));
    let right_rank = rank_order(&purust_json_rank(right));
    if left_rank != right_rank {
        return if left_rank < right_rank {
            PurustJsonOrdering::Less
        } else {
            PurustJsonOrdering::Greater
        };
    }
    match left.resolve() {
        crate::Value::Null => PurustJsonOrdering::Equal,
        crate::Value::Bool(left_flag) => match right.resolve() {
            crate::Value::Bool(right_flag) if left_flag == right_flag => PurustJsonOrdering::Equal,
            _ if !*left_flag => PurustJsonOrdering::Less,
            _ => PurustJsonOrdering::Greater,
        },
        crate::Value::Int(left_number) => {
            let left_number = *left_number as f64;
            let right_number = match right.resolve() {
                crate::Value::Int(n) => *n as f64,
                crate::Value::Number(n) => *n,
                _ => unreachable!(),
            };
            if left_number == right_number {
                PurustJsonOrdering::Equal
            } else if left_number < right_number {
                PurustJsonOrdering::Less
            } else {
                PurustJsonOrdering::Greater
            }
        }
        crate::Value::Number(left_number) => {
            let left_number = *left_number;
            let right_number = match right.resolve() {
                crate::Value::Int(n) => *n as f64,
                crate::Value::Number(n) => *n,
                _ => unreachable!(),
            };
            if left_number == right_number {
                PurustJsonOrdering::Equal
            } else if left_number < right_number {
                PurustJsonOrdering::Less
            } else {
                PurustJsonOrdering::Greater
            }
        }
        crate::Value::String(_) | crate::Value::Char(_) => {
            let left_units = purust_json_string_units(left);
            let right_units = purust_json_string_units(right);
            if left_units == right_units {
                PurustJsonOrdering::Equal
            } else if left_units < right_units {
                PurustJsonOrdering::Less
            } else {
                PurustJsonOrdering::Greater
            }
        }
        crate::Value::Array(_) | crate::Value::IntArray(_) => {
            let left_values = purust_json_array_values(left);
            let right_values = purust_json_array_values(right);
            for (left_value, right_value) in left_values.iter().zip(right_values.iter()) {
                match purust_json_compare_values(left_value, right_value) {
                    PurustJsonOrdering::Equal => {}
                    other => return other,
                }
            }
            if left_values.len() == right_values.len() {
                PurustJsonOrdering::Equal
            } else if left_values.len() < right_values.len() {
                PurustJsonOrdering::Less
            } else {
                PurustJsonOrdering::Greater
            }
        }
        crate::Value::Class(_) => {
            let left_object = purust_json_object(left);
            let right_object = purust_json_object(right);
            let mut left_entries = left_object.entries();
            let mut right_entries = right_object.entries();
            if left_entries.len() != right_entries.len() {
                return if left_entries.len() < right_entries.len() {
                    PurustJsonOrdering::Less
                } else {
                    PurustJsonOrdering::Greater
                };
            }
            left_entries.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
            right_entries.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
            for ((left_key, left_value), (right_key, right_value)) in
                left_entries.iter().zip(right_entries.iter())
            {
                if left_key != right_key {
                    return if left_key < right_key {
                        PurustJsonOrdering::Less
                    } else {
                        PurustJsonOrdering::Greater
                    };
                }
                match purust_json_compare_values(left_value, right_value) {
                    PurustJsonOrdering::Equal => {}
                    other => return other,
                }
            }
            PurustJsonOrdering::Equal
        }
        _ => unreachable!(),
    }
}

pub fn Data_Argonaut_Core__compare() -> crate::UnknownType {
    crate::Value::Func5(purust_core::Func5::Static(|eq, gt, lt, left, right| {
        match purust_json_compare_values(&left, &right) {
            PurustJsonOrdering::Less => lt,
            PurustJsonOrdering::Equal => eq,
            PurustJsonOrdering::Greater => gt,
        }
    }))
}

// ---------------------------------------------------------------------------
// JSON text parsing (V8-compatible messages) and canonical fingerprints used
// by the benchmark driver.

struct PurustJsonParser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    index: usize,
}

enum PurustJsonFrame {
    Array(Vec<crate::UnknownType>),
    Object(purust_core::RecordFields, std::rc::Rc<str>),
}

impl<'a> PurustJsonParser<'a> {
    fn fail(&self, reason: &str) -> String {
        // Positions count UTF-16 code units, like the V8 messages. The
        // encoded representation maps one unit to one char, so the prefix
        // length is the position.
        format!(
            "{} in JSON at position {}",
            reason,
            self.text[..self.index].chars().count()
        )
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(9 | 10 | 13 | 32)) {
            self.index += 1;
        }
    }

    fn take(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if !self.take(b'"') {
            return Err(self.fail("Expected double-quoted property name"));
        }
        // Fast path for the common case: scan the encoded bytes up to the
        // closing quote and copy the range in one go. Escapes, control
        // characters and the unterminated case fall back to the escaping
        // loop.
        let start = self.index;
        let mut end = start;
        while let Some(byte) = self.bytes.get(end).copied() {
            match byte {
                b'"' => {
                    let value = self.text[start..end].to_owned();
                    self.index = end + 1;
                    return Ok(value);
                }
                b'\\' | 0..=0x1f => break,
                _ => end += 1,
            }
        }
        self.index = start;
        self.string_escaped()
    }

    // Object keys are stored as shared strings: build the Rc from the byte
    // range directly instead of allocating a String first.
    fn string_key(&mut self) -> Result<std::rc::Rc<str>, String> {
        if !self.take(b'"') {
            return Err(self.fail("Expected double-quoted property name"));
        }
        let start = self.index;
        let mut end = start;
        while let Some(byte) = self.bytes.get(end).copied() {
            match byte {
                b'"' => {
                    let key: std::rc::Rc<str> = std::rc::Rc::from(&self.text[start..end]);
                    self.index = end + 1;
                    return Ok(key);
                }
                b'\\' | 0..=0x1f => break,
                _ => end += 1,
            }
        }
        self.index = start;
        Ok(std::rc::Rc::from(self.string_escaped()?))
    }

    fn string_escaped(&mut self) -> Result<String, String> {
        let mut result = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.fail("Unterminated string"));
            };
            match byte {
                b'"' => {
                    self.index += 1;
                    return Ok(result);
                }
                0..=0x1f => return Err(self.fail("Bad control character in string literal")),
                b'\\' => {
                    self.index += 1;
                    let Some(escape) = self.peek() else {
                        return Err(self.fail("Unterminated string"));
                    };
                    self.index += 1;
                    match escape {
                        b'"' | b'/' | b'\\' => result.push(escape as char),
                        b'b' => result.push('\u{8}'),
                        b'f' => result.push('\u{c}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            let mut code: u16 = 0;
                            for _ in 0..4 {
                                let digit = match self.peek() {
                                    Some(c @ b'0'..=b'9') => c - b'0',
                                    Some(c @ b'A'..=b'F') => c - b'A' + 10,
                                    Some(c @ b'a'..=b'f') => c - b'a' + 10,
                                    _ => return Err(self.fail("Bad Unicode escape")),
                                };
                                self.index += 1;
                                code = code * 16 + digit as u16;
                            }
                            // Never normalize a pair or replace a lone
                            // surrogate: store the unit as-is.
                            result.push(purust_core::purust_char_from_code_unit(code));
                        }
                        _ => return Err(self.fail("Bad escaped character")),
                    }
                }
                _ => {
                    // One encoded char is one code unit.
                    let character = self.text[self.index..]
                        .chars()
                        .next()
                        .expect("a byte at a char boundary starts a char");
                    self.index += character.len_utf8();
                    result.push(character);
                }
            }
        }
    }

    fn number(&mut self) -> Result<crate::UnknownType, String> {
        let start = self.index;
        self.take(b'-');
        if !self.take(b'0') {
            if !matches!(self.peek(), Some(b'1'..=b'9')) {
                return Err(self.fail("Invalid number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.index += 1;
            }
        }
        if self.take(b'.') {
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.fail("Unterminated fractional number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.index += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.index += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.fail("Exponent part is missing a number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.index += 1;
            }
        }
        let length = self.index - start;
        // The scanned bytes are ASCII: parse from a stack buffer so ordinary
        // numbers allocate nothing, and keep a heap fallback for absurdly
        // long literals.
        let parsed = if length <= 40 {
            let mut buffer = [0u8; 40];
            buffer[..length].copy_from_slice(&self.bytes[start..self.index]);
            std::str::from_utf8(&buffer[..length])
                .ok()
                .and_then(|spelling| spelling.parse::<f64>().ok())
        } else {
            self.text[start..self.index].parse::<f64>().ok()
        };
        // JSON.parse uses IEEE-754 even for integers, including overflow and -0.
        match parsed {
            Some(number) => Ok(crate::Value::Number(number)),
            None => Err(self.fail("Invalid number")),
        }
    }

    fn property(&mut self) -> Result<std::rc::Rc<str>, String> {
        self.whitespace();
        let key = self.string_key()?;
        self.whitespace();
        if !self.take(b':') {
            return Err(self.fail("Expected ':' after property name"));
        }
        Ok(key)
    }

    fn parse(mut self) -> Result<crate::UnknownType, String> {
        // An explicit stack avoids aborting the process on deeply nested input.
        let mut stack = Vec::new();
        loop {
            self.whitespace();
            let mut value = match self.peek() {
                Some(b'"') => crate::Value::String(self.string()?),
                Some(b'-' | b'0'..=b'9') => self.number()?,
                Some(b'n' | b't' | b'f') => {
                    let (literal, value) = match self.peek() {
                        Some(b'n') => ("null", crate::Value::Null),
                        Some(b't') => ("true", crate::Value::Bool(true)),
                        _ => ("false", crate::Value::Bool(false)),
                    };
                    for &expected in literal.as_bytes() {
                        if !self.take(expected) {
                            return Err(self.fail("Unexpected token"));
                        }
                    }
                    value
                }
                Some(b'[') => {
                    self.index += 1;
                    self.whitespace();
                    if !self.take(b']') {
                        stack.push(PurustJsonFrame::Array(Vec::new()));
                        continue;
                    }
                    crate::Value::Array(Rc::new(Vec::new()))
                }
                Some(b'{') => {
                    self.index += 1;
                    self.whitespace();
                    if !self.take(b'}') {
                        let key = self.property()?;
                        stack.push(PurustJsonFrame::Object(
                            purust_core::RecordFields::new(),
                            key,
                        ));
                        continue;
                    }
                    crate::Value::Class(Rc::new(Rc::new(Purs_Foreign_Object::Object::empty())))
                }
                None => return Err(self.fail("Unexpected end of JSON input")),
                _ => return Err(self.fail("Unexpected token")),
            };
            loop {
                self.whitespace();
                value = match stack.last_mut() {
                    None => {
                        if self.peek().is_some() {
                            return Err(self.fail("Unexpected non-whitespace character after JSON"));
                        }
                        return Ok(value);
                    }
                    Some(PurustJsonFrame::Array(values)) => {
                        values.push(value);
                        if self.take(b',') {
                            break;
                        }
                        if !self.take(b']') {
                            return Err(self.fail("Expected ',' or ']' after array element"));
                        }
                        let Some(PurustJsonFrame::Array(values)) = stack.pop() else {
                            unreachable!()
                        };
                        crate::Value::Array(Rc::new(values))
                    }
                    Some(PurustJsonFrame::Object(fields, key)) => {
                        fields.insert_shared(std::mem::take(key), value);
                        if self.take(b',') {
                            *key = self.property()?;
                            break;
                        }
                        if !self.take(b'}') {
                            return Err(self.fail("Expected ',' or '}' after property value"));
                        }
                        let Some(PurustJsonFrame::Object(fields, _)) = stack.pop() else {
                            unreachable!()
                        };
                        crate::Value::Class(Rc::new(Rc::new(
                            Purs_Foreign_Object::Object::from_entries_shared(fields.into_entries_shared()),
                        )))
                    }
                };
            }
        }
    }
}

pub fn purust_json_parse_text(text: &str) -> Result<crate::UnknownType, String> {
    PurustJsonParser {
        bytes: text.as_bytes(),
        text,
        index: 0,
    }
    .parse()
}

// ---------------------------------------------------------------------------
// Canonical JSON and SHA-256 for the benchmark oracle. The canonical form
// matches Go's `json.Marshal` and the JavaScript driver: sorted keys and the
// HTML/line-separator escapes.

fn purust_canonical_string(value: &str) -> String {
    purust_json_quote(value)
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn purust_canonical_value(value: &crate::UnknownType) -> String {
    match value.resolve() {
        crate::Value::Null => "null".into(),
        crate::Value::Int(n) => n.to_string(),
        crate::Value::Number(n) => purust_json_number(*n),
        crate::Value::Bool(b) => b.to_string(),
        crate::Value::String(s) => purust_canonical_string(s),
        crate::Value::Char(c) => purust_canonical_string(&c.to_string()),
        crate::Value::IntArray(values) => format!(
            "[{}]",
            values.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
        ),
        crate::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(purust_canonical_value)
                .collect::<Vec<_>>()
                .join(",")
        ),
        crate::Value::Class(native) => match native.downcast_ref::<Rc<Purs_Foreign_Object::Object>>() {
            Some(object) => {
                let mut entries = object.entries();
                entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                let parts: Vec<_> = entries
                    .into_iter()
                    .map(|(key, value)| {
                        format!("{}:{}", purust_canonical_string(&key), purust_canonical_value(&value))
                    })
                    .collect();
                format!("{{{}}}", parts.join(","))
            }
            None => panic!("Data.Argonaut.Core: canonical form of an unsupported value"),
        },
        _ => panic!("Data.Argonaut.Core: canonical form of a non-JSON value"),
    }
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn purust_sha256_hex(input: &[u8]) -> String {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = input.to_vec();
    let bit_length = (input.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    for chunk in message.chunks(64) {
        let mut schedule = [0u32; 64];
        for (index, word) in chunk.chunks(4).enumerate() {
            schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }
        let mut working = state;
        for index in 0..64 {
            let s1 = working[4].rotate_right(6) ^ working[4].rotate_right(11) ^ working[4].rotate_right(25);
            let choice = (working[4] & working[5]) ^ (!working[4] & working[6]);
            let temp1 = working[7]
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(SHA256_K[index])
                .wrapping_add(schedule[index]);
            let s0 = working[0].rotate_right(2) ^ working[0].rotate_right(13) ^ working[0].rotate_right(22);
            let majority = (working[0] & working[1]) ^ (working[0] & working[2]) ^ (working[1] & working[2]);
            let temp2 = s0.wrapping_add(majority);
            working[7] = working[6];
            working[6] = working[5];
            working[5] = working[4];
            working[4] = working[3].wrapping_add(temp1);
            working[3] = working[2];
            working[2] = working[1];
            working[1] = working[0];
            working[0] = temp1.wrapping_add(temp2);
        }
        for index in 0..8 {
            state[index] = state[index].wrapping_add(working[index]);
        }
    }
    state.iter().map(|word| format!("{:08x}", word)).collect()
}

pub fn purust_canonical_hash(text: &str) -> String {
    match purust_json_parse_text(text) {
        Ok(value) => {
            // The canonical text lives in the runtime's encoded string domain;
            // hash the ordinary UTF-8 bytes the other drivers hash.
            let bytes = purust_core::purust_string_to_utf8_lossy(&purust_canonical_value(&value));
            purust_sha256_hex(bytes.as_bytes())
        }
        Err(error) => panic!("Data.Argonaut.Core: canonical hash input is not JSON: {}", error),
    }
}
