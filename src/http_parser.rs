use http_muncher::{ParserHandler};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std;


pub struct HttpParser {
    pub current_key: Option<String>,
    pub headers: Arc<Mutex<HashMap<String, String>>>
}

impl ParserHandler for HttpParser {
    fn on_header_field(&mut self, s: &[u8]) -> bool {
        self.current_key = Some(std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_header_value(&mut self, s: &[u8]) -> bool {
        self.headers.lock().unwrap()
            .insert(self.current_key.clone().unwrap(),
                    std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_headers_complete(&mut self) -> bool {
        false
    }
}
