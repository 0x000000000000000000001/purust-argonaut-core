// The uncurried parser keeps the JavaScript contract: a parser failure calls
// the first callback with the V8-compatible message, a success calls the
// second with the parsed Json value.
pub fn Data_Argonaut_Parser__jsonParser() -> crate::UnknownType {
    crate::Value::Func3(purust_core::Func3::Static(|fail, succ, text| {
        let input = text.unwrap_string();
        match Purs_Data_Argonaut_Core::purust_json_parse_text(&input) {
            Ok(value) => succ.unwrap_func1()(value),
            Err(message) => fail.unwrap_func1()(crate::Value::String(message)),
        }
    }))
}
