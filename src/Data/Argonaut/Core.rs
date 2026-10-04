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
    value.is_array().then(|| value.array_iter().collect())
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
        crate::Value::NativeArray(_) => format!("[{}]", value.array_iter()
            .map(|item| purust_json_value(&item)).collect::<Vec<_>>().join(",")),
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
        crate::Value::Array(_) | crate::Value::IntArray(_) | crate::Value::NativeArray(_) => value.clone(),
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
        crate::Value::Array(_) | crate::Value::IntArray(_) | crate::Value::NativeArray(_) => on_array(value.clone()),
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
        crate::Value::Array(_) | crate::Value::IntArray(_) | crate::Value::NativeArray(_) => on_array(value.clone()),
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
        crate::Value::Array(_) | crate::Value::IntArray(_) | crate::Value::NativeArray(_) => JsonRank::Array,
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
        crate::Value::NativeArray(_) => value.array_iter().collect(),
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
        crate::Value::Array(_) | crate::Value::IntArray(_) | crate::Value::NativeArray(_) => {
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
        let end = purust_json_plain_end(self.bytes, start);
        if self.bytes.get(end) == Some(&b'"') {
            let value = self.text[start..end].to_owned();
            self.index = end + 1;
            return Ok(value);
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
        let end = purust_json_plain_end(self.bytes, start);
        if self.bytes.get(end) == Some(&b'"') {
            let key: std::rc::Rc<str> = std::rc::Rc::from(&self.text[start..end]);
            self.index = end + 1;
            return Ok(key);
        }
        self.index = start;
        Ok(std::rc::Rc::from(self.string_escaped()?))
    }

    fn string_escaped(&mut self) -> Result<String, String> {
        self.string_escaped_into(String::new())
    }

    fn string_escaped_into(&mut self, mut result: String) -> Result<String, String> {
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
                    // The delimiter/control bytes are ASCII, hence these
                    // offsets remain UTF-8 boundaries in the encoded UTF-16
                    // string. Copy a whole plain run without normalizing it.
                    let end = purust_json_plain_end(self.bytes, self.index);
                    result.push_str(&self.text[self.index..end]);
                    self.index = end;
                }
            }
        }
    }

    fn number_spelling(&mut self) -> Result<&str, String> {
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
        Ok(&self.text[start..self.index])
    }

    fn number(&mut self) -> Result<crate::UnknownType, String> {
        let parsed = self.number_spelling()?.parse::<f64>().ok();
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

// Eight independent byte lanes: only quote, backslash and controls terminate
// a plain run. High UTF-8 bytes of the internal UTF-16 encoding are copied as
// opaque bytes. Safe slice loads never read past the input, including tails.
#[inline]
fn purust_json_plain_end(bytes: &[u8], mut index: usize) -> usize {
    const ONES: u64 = 0x0101010101010101;
    const HIGH: u64 = 0x8080808080808080;
    while index + 8 <= bytes.len() {
        let word = u64::from_ne_bytes(bytes[index..index + 8].try_into().unwrap());
        let quote = word ^ (ONES * b'"' as u64);
        let slash = word ^ (ONES * b'\\' as u64);
        let special = (quote.wrapping_sub(ONES) & !quote)
            | (slash.wrapping_sub(ONES) & !slash)
            | (word.wrapping_sub(ONES * 0x20) & !word);
        if special & HIGH != 0 { break; }
        index += 8;
    }
    while let Some(&byte) = bytes.get(index) {
        if byte < 0x20 || byte == b'"' || byte == b'\\' { break; }
        index += 1;
    }
    index
}

#[cfg(test)]
mod purust_json_scan_tests {
    #[test]
    fn every_byte_at_every_lane_and_tail_matches_scalar_scanning() {
        for start in 0..16 {
            for length in 0..40 {
                for lane in 0..length {
                    for byte in 0..=255u8 {
                        let mut input = vec![0xaa; start + length];
                        input[start + lane] = byte;
                        let expected = (start..input.len()).find(|&index| {
                            let b = input[index]; b < 0x20 || b == b'"' || b == b'\\'
                        }).unwrap_or(input.len());
                        assert_eq!(super::purust_json_plain_end(&input, start), expected);
                    }
                }
            }
        }
        // Multiple specials and near-matches exercise cross-lane borrow,
        // including chunk boundaries and high bytes in UTF-16 storage.
        let alphabet = [0, 1, 0x1f, 0x20, 0x21, b'"', b'#', b'\\', b']', 0x7f, 0x80, 0xff];
        let mut seed = 0x51ca77e5u32;
        for length in 0..128 {
            for _ in 0..128 {
                let input: Vec<_> = (0..length).map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    alphabet[seed as usize % alphabet.len()]
                }).collect();
                for start in 0..=input.len() {
                    let expected = (start..input.len()).find(|&index| {
                        let b = input[index]; b < 0x20 || b == b'"' || b == b'\\'
                    }).unwrap_or(input.len());
                    assert_eq!(super::purust_json_plain_end(&input, start), expected);
                }
            }
        }
    }
}

// Validated, call-local offset tape for compiler-generated decoders. It never
// escapes as Json. Final strings own their storage, and failures use the
// ordinary parser/decoder to preserve its exact diagnostics. The bounded
// recursive scanner declines deep input; the ordinary parser is iterative.
const PURUST_JSON_ESCAPED: u32 = 1 << 31;

#[derive(Clone, Copy, Default)]
struct PurustJsonToken { start: u32, end: u32, next: u32 }

pub struct PurustJsonDocument<'a> {
    text: &'a str,
    tokens: Vec<PurustJsonToken>,
}

#[derive(Clone, Copy)]
pub struct PurustJsonCursor<'a> {
    document: &'a PurustJsonDocument<'a>,
    index: usize,
}

struct PurustJsonIndexer<'a> {
    parser: PurustJsonParser<'a>,
    tokens: Vec<PurustJsonToken>,
}

impl<'a> PurustJsonDocument<'a> {
    pub fn parse(text: &'a str) -> Option<Self> {
        if text.len() >= PURUST_JSON_ESCAPED as usize { return None; }
        let mut scanner = PurustJsonIndexer {
            parser: PurustJsonParser { text, bytes: text.as_bytes(), index: 0 },
            tokens: Vec::with_capacity(text.len() / 6 + 8),
        };
        scanner.value(0)?;
        scanner.parser.whitespace();
        if scanner.parser.index != text.len() { return None; }
        Some(Self { text, tokens: scanner.tokens })
    }
    pub fn root(&self) -> PurustJsonCursor<'_> { PurustJsonCursor { document: self, index: 0 } }
}

impl PurustJsonIndexer<'_> {
    fn string(&mut self) -> Option<()> {
        let start = self.parser.index;
        if !self.parser.take(b'"') { return None; }
        let mut escaped = false;
        loop {
            self.parser.index = purust_json_plain_end(self.parser.bytes, self.parser.index);
            let byte = self.parser.peek()?;
            self.parser.index += 1;
            match byte {
                b'"' => {
                    let index = self.tokens.len();
                    self.tokens.push(PurustJsonToken {
                        start: start as u32,
                        end: self.parser.index as u32 | if escaped { PURUST_JSON_ESCAPED } else { 0 },
                        next: (index + 1) as u32,
                    });
                    return Some(());
                }
                0..=0x1f => return None,
                b'\\' => {
                    escaped = true;
                    let escape = self.parser.peek()?;
                    self.parser.index += 1;
                    match escape {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                        b'u' => for _ in 0..4 {
                            if !self.parser.peek()?.is_ascii_hexdigit() { return None; }
                            self.parser.index += 1;
                        },
                        _ => return None,
                    }
                }
                _ => {}
            }
        }
    }

    fn value(&mut self, depth: usize) -> Option<()> {
        self.parser.whitespace();
        let start = self.parser.index;
        let index = self.tokens.len();
        let first = self.parser.peek()?;
        match first {
            b'"' => return self.string(),
            b'{' | b'[' => {
                if depth >= 128 { return None; }
                let object = first == b'{';
                let close = if object { b'}' } else { b']' };
                self.parser.index += 1;
                self.tokens.push(PurustJsonToken { start: start as u32, ..Default::default() });
                self.parser.whitespace();
                let mut count = 0;
                let mut last_key = 0;
                if !self.parser.take(close) {
                    loop {
                        if object {
                            let key = self.tokens.len();
                            self.string()?;
                            self.tokens[key].next = last_key;
                            last_key = key as u32;
                            self.parser.whitespace();
                            if !self.parser.take(b':') { return None; }
                        }
                        self.value(depth + 1)?;
                        count += 1;
                        self.parser.whitespace();
                        if self.parser.take(close) { break; }
                        if !self.parser.take(b',') { return None; }
                        self.parser.whitespace();
                    }
                }
                self.tokens[index].end = if object { last_key } else { count };
                self.tokens[index].next = self.tokens.len() as u32;
                return Some(());
            }
            b'n' | b't' | b'f' => {
                let literal: &[u8] = match first { b'n' => b"null", b't' => b"true", _ => b"false" };
                for &byte in literal { if !self.parser.take(byte) { return None; } }
            }
            b'-' | b'0'..=b'9' => { self.parser.number_spelling().ok()?; }
            _ => return None,
        }
        self.tokens.push(PurustJsonToken { start: start as u32, end: self.parser.index as u32, next: (index + 1) as u32 });
        Some(())
    }
}

impl<'a> PurustJsonCursor<'a> {
    pub fn kind(self) -> u8 { self.document.text.as_bytes()[self.document.tokens[self.index].start as usize] }
    fn string(self) -> Option<std::borrow::Cow<'a, str>> {
        if self.kind() != b'"' { return None; }
        let token = self.document.tokens[self.index];
        if token.end & PURUST_JSON_ESCAPED == 0 {
            return Some(std::borrow::Cow::Borrowed(&self.document.text[token.start as usize + 1..token.end as usize - 1]));
        }
        let mut parser = PurustJsonParser { text: self.document.text, bytes: self.document.text.as_bytes(), index: token.start as usize + 1 };
        // Every escape expands to at most its encoded spelling's byte length.
        // The validated tape supplies the bound without another string scan.
        let capacity = (token.end & !PURUST_JSON_ESCAPED) as usize - parser.index - 1;
        Some(std::borrow::Cow::Owned(parser.string_escaped_into(String::with_capacity(capacity)).ok()?))
    }
    pub fn scalar(self, kind: &str) -> Option<crate::UnknownType> {
        match kind {
            "String" => Some(crate::Value::String(self.string()?.into_owned())),
            "Boolean" => match self.kind() { b't' => Some(crate::Value::Bool(true)), b'f' => Some(crate::Value::Bool(false)), _ => None },
            "Int" | "Number" => {
                if !matches!(self.kind(), b'-' | b'0'..=b'9') { return None; }
                let token = self.document.tokens[self.index];
                let spelling = &self.document.text[token.start as usize..token.end as usize];
                if kind == "Int" {
                    if let Some(integer) = purust_json_small_int(spelling) { return Some(crate::Value::Int(integer)); }
                }
                let number = spelling.parse::<f64>().ok()?;
                if kind == "Number" { return Some(crate::Value::Number(number)); }
                if number.is_finite() && number.fract() == 0.0 && (-2147483648.0..=2147483647.0).contains(&number) {
                    Some(crate::Value::Int(number as i64))
                } else { None }
            }
            // Escaping Json subtrees keep ordinary materialization and aliases.
            _ => None,
        }
    }
    pub fn string_eq(&self, value: &str) -> Option<bool> {
        Some(self.string()?.as_ref() == value)
    }
    pub fn fields<const N: usize>(self, keys: [&str; N]) -> Option<[Option<Self>; N]> {
        if self.kind() != b'{' { return None; }
        let mut fields = [None; N];
        let mut at = self.document.tokens[self.index].end as usize;
        while at != 0 {
            let cursor = Self { document: self.document, index: at };
            let key = cursor.string()?;
            if let Some(slot) = keys.iter().position(|expected| *expected == key.as_ref()) {
                // Reverse traversal gives the last normalized spelling of a key.
                if fields[slot].is_none() { fields[slot] = Some(Self { document: self.document, index: at + 1 }); }
            }
            at = self.document.tokens[at].next as usize;
        }
        Some(fields)
    }
    pub fn array(self) -> Option<PurustJsonCursorItems<'a>> {
        if self.kind() != b'[' { return None; }
        Some(PurustJsonCursorItems { cursor: Self { index: self.index + 1, ..self }, remaining: self.document.tokens[self.index].end as usize })
    }
    /// Materialize a validated subtree into the ordinary boxed JSON
    /// representation. Compiler FFI uses this for cold fields that keep their
    /// generated decoders; the result is built exactly like
    /// `purust_json_parse_text` builds it: object entries in document order
    /// through `from_entries_shared` (duplicates keep the last value), numbers
    /// as `Value::Number`, strings in the encoded UTF-16 domain, arrays as
    /// shared vectors.
    pub fn materialize(self) -> Option<crate::UnknownType> {
        match self.kind() {
            b'"' => Some(crate::Value::String(self.string()?.into_owned())),
            b'{' => {
                let mut entries: Vec<(std::rc::Rc<str>, crate::UnknownType)> = Vec::new();
                // Key.next walks the object backwards from its last key.
                let mut at = self.document.tokens[self.index].end as usize;
                while at != 0 {
                    let key = Self { document: self.document, index: at }.string()?;
                    let value = Self { document: self.document, index: at + 1 }.materialize()?;
                    entries.push((std::rc::Rc::from(key.as_ref()), value));
                    at = self.document.tokens[at].next as usize;
                }
                entries.reverse();
                Some(crate::Value::Class(std::rc::Rc::new(std::rc::Rc::new(
                    Purs_Foreign_Object::Object::from_entries_shared(entries),
                ))))
            }
            b'[' => {
                let items = self.array()?;
                let mut values = Vec::with_capacity(items.len());
                for item in items { values.push(item.materialize()?); }
                Some(crate::Value::Array(std::rc::Rc::new(values)))
            }
            b't' => Some(crate::Value::Bool(true)),
            b'f' => Some(crate::Value::Bool(false)),
            b'n' => Some(crate::Value::Null),
            _ => Some(self.scalar("Number")?),
        }
    }
}

pub struct PurustJsonCursorItems<'a> { cursor: PurustJsonCursor<'a>, remaining: usize }
impl<'a> std::iter::Iterator for PurustJsonCursorItems<'a> {
    type Item = PurustJsonCursor<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 { return None; }
        let cursor = self.cursor;
        self.cursor.index = cursor.document.tokens[cursor.index].next as usize;
        self.remaining -= 1;
        Some(cursor)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.remaining, Some(self.remaining)) }
}
impl std::iter::ExactSizeIterator for PurustJsonCursorItems<'_> {}

// Only already-validated, plain decimal Int spellings take this path. Decimal
// points, exponents and all other cases retain the ordinary IEEE-754 conversion
// and range check (including values which round to an integral Number).
fn purust_json_small_int(spelling: &str) -> Option<i64> {
    let bytes = spelling.as_bytes();
    let negative = bytes.first() == Some(&b'-');
    let digits = if negative { &bytes[1..] } else { bytes };
    if digits.is_empty() || digits.len() > 10 { return None; }
    let magnitude = digits.iter().try_fold(0i64, |value, &byte| {
        byte.is_ascii_digit().then(|| value * 10 + (byte - b'0') as i64)
    })?;
    let value = if negative { -magnitude } else { magnitude };
    (-2147483648..=2147483647).contains(&value).then_some(value)
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
        crate::Value::NativeArray(_) => format!("[{}]", value.array_iter()
            .map(|item| purust_canonical_value(&item)).collect::<Vec<_>>().join(",")),
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
