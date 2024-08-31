use core::panic;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Map, Value};

#[macro_export]
macro_rules! outputjson {
    ($output:expr) => {
        #[cfg(not(any(target_os = "wasi", unix)))]
        {
            compile_error!("outputjson macro is not supported on this platform");
        }

        #[cfg(target_os = "wasi")]
        {
            #[repr(C)]
            struct Ciovec {
                buf: *const u8,
                buf_len: usize,
            }

            #[link(wasm_import_module = "wasi_snapshot_preview1")]
            extern "C" {
                fn fd_write(
                    fd: u32,
                    iovs: *const Ciovec,
                    iovs_len: usize,
                    nwritten: *mut usize,
                ) -> u32;
            }

            let output_str = format!("{}\n", serde_json::to_string($output)?);
            let iovec = Ciovec {
                buf: output_str.as_ptr(),
                buf_len: output_str.len(),
            };
            let mut nwritten = 0;

            unsafe { fd_write(3, &iovec, 1, &mut nwritten) };
        }

        #[cfg(unix)]
        {
            let output_str = serde_json::to_string($output)?;
            use std::fs::File;
            use std::io::Write;
            use std::os::unix::io::FromRawFd;
            let mut file = unsafe { File::from_raw_fd(3) };
            writeln!(file, "{}", output_str)?;
        }
    };
}

pub enum Data<T: Serialize + DeserializeOwned + Default> {
    Input(T),
    Stream(String, Value),
    Continue,
}

// this struct will be used to iterate over the data
pub struct DataIterator<'a, T: Serialize + DeserializeOwned + Default> {
    processor: &'a mut StreamProcessor<T>,
    json: Map<String, Value>,
    index: usize,
}

impl<'a, T: Serialize + DeserializeOwned + Default> Iterator for DataIterator<'a, T> {
    type Item = Data<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // we need to look at the key based on the index and process it then increment the index
        // if the index is greater than the length of the keys then we should return None

        let key = self.json.keys().nth(self.index);
        self.index += 1;

        match key {
            Some(key) => {
                if self.processor.fields.contains(&key) {
                    if self.processor.map.contains_key(key) {
                        panic!("Duplicate input key found: {}", key);
                    } else {
                        if let Some(value) = self.json.get(key) {
                            self.processor.map.insert(key.to_string(), value.clone());
                        } else {
                            panic!("Key not found in json: {}", key);
                        }
                    }
                    if self.processor.fields.len() == self.processor.map.len() {
                        match serde_json::from_value::<T>(Value::Object(self.processor.map.clone()))
                        {
                            Ok(input) => Some(Data::Input(input)),
                            Err(e) => {
                                panic!("Error converting map to struct: {}", e);
                            }
                        }
                    } else {
                        Some(Data::Continue)
                    }
                } else {
                    Some(Data::Stream(
                        key.to_string(),
                        self.json.get(key).unwrap().clone(),
                    ))
                }
            }
            None => None,
        }
    }
}

pub struct StreamProcessor<T: Serialize + DeserializeOwned + Default> {
    fields: Vec<String>,
    map: Map<String, Value>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned + Default> StreamProcessor<T> {
    pub fn new() -> Self {
        let fields = match serde_json::to_value(&T::default()).unwrap() {
            Value::Object(map) => map.keys().map(|k| k.to_string()).collect(),
            _ => vec![],
        };

        StreamProcessor {
            fields,
            map: Map::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn process(&mut self, json: Value) -> DataIterator<T> {
        // we need to check if the json is an object
        // if not we should panic
        match json {
            Value::Object(map) => DataIterator {
                processor: self,
                json: map,
                index: 0,
            },
            value => {
                panic!(
                    "StreamProcessor::process expects an object, got {}.",
                    match value {
                        Value::Bool(_) => "a boolean",
                        Value::Number(_) => "a number",
                        Value::String(_) => "a string",
                        Value::Array(_) => "an array",
                        _ => "null",
                    }
                );
            }
        }
    }
}
