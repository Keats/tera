use crate::value::{Key, Map, ValueInner};
use crate::{HashMap, Value};
use std::sync::Arc;

/// Lazy iterator for for-loop values that only clones the current item
#[derive(Debug)]
pub(crate) enum ForLoopIterator {
    Array {
        arr: Arc<Vec<Value>>,
        index: usize,
    },
    Map {
        map: Arc<Map>,
        keys: std::vec::IntoIter<Key<'static>>,
    },
    #[cfg(not(feature = "unicode"))]
    String {
        content: Arc<str>,
        current_pos: usize,
    },
    Bytes {
        bytes: Arc<Vec<u8>>,
        index: usize,
    },
    #[cfg(feature = "unicode")]
    Graphemes {
        graphemes: Vec<String>,
        index: usize,
    },
}

impl Iterator for ForLoopIterator {
    type Item = (Option<Value>, Value);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ForLoopIterator::Array { arr, index } => {
                if *index < arr.len() {
                    let value = arr[*index].clone();
                    *index += 1;
                    Some((None, value))
                } else {
                    None
                }
            }

            ForLoopIterator::Map { map, keys } => keys.next().map(|key| {
                let value = map[&key].clone();
                let key_value = key.into();
                (Some(key_value), value)
            }),

            #[cfg(not(feature = "unicode"))]
            ForLoopIterator::String {
                content,
                current_pos,
            } => {
                if *current_pos >= content.len() {
                    return None;
                }

                let remaining = &content[*current_pos..];
                if let Some((char_end, _)) = remaining.char_indices().nth(1) {
                    let char_str = &remaining[..char_end];
                    *current_pos += char_end;
                    Some((None, Value::from(char_str)))
                } else {
                    // Last character
                    let char_str = remaining;
                    *current_pos = content.len();
                    Some((None, Value::from(char_str)))
                }
            }

            ForLoopIterator::Bytes { bytes, index } => {
                if *index < bytes.len() {
                    let value = Value::from(bytes[*index] as u64);
                    *index += 1;
                    Some((None, value))
                } else {
                    None
                }
            }

            #[cfg(feature = "unicode")]
            ForLoopIterator::Graphemes { graphemes, index } => {
                if *index >= graphemes.len() {
                    return None;
                }
                let grapheme = &graphemes[*index];
                *index += 1;
                Some((None, Value::from(grapheme.clone())))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            ForLoopIterator::Array { arr, index } => Self::indexed_size_hint(arr.len(), *index),
            ForLoopIterator::Map { keys, .. } => keys.size_hint(),
            #[cfg(not(feature = "unicode"))]
            ForLoopIterator::String {
                content,
                current_pos,
            } => {
                let remaining = content[*current_pos..].chars().count();
                (remaining, Some(remaining))
            }
            ForLoopIterator::Bytes { bytes, index } => Self::indexed_size_hint(bytes.len(), *index),
            #[cfg(feature = "unicode")]
            ForLoopIterator::Graphemes { graphemes, index } => {
                let remaining = graphemes.len() - *index;
                (remaining, Some(remaining))
            }
        }
    }
}

impl ForLoopIterator {
    fn indexed_size_hint(len: usize, index: usize) -> (usize, Option<usize>) {
        let remaining = len - index;
        (remaining, Some(remaining))
    }

    fn create_string_iterator(content: Arc<str>) -> ForLoopIterator {
        #[cfg(feature = "unicode")]
        {
            use unicode_segmentation::UnicodeSegmentation;
            let graphemes: Vec<String> = content.graphemes(true).map(|g| g.to_string()).collect();
            ForLoopIterator::Graphemes {
                graphemes,
                index: 0,
            }
        }
        #[cfg(not(feature = "unicode"))]
        {
            ForLoopIterator::String {
                content,
                current_pos: 0,
            }
        }
    }
}

/// Create a lazy iterator for for-loop iteration that only clones the current item
pub(crate) fn create_for_loop_iterator(value: &Value) -> Option<ForLoopIterator> {
    match &value.inner {
        ValueInner::Array(arr) => Some(ForLoopIterator::Array {
            arr: Arc::clone(arr),
            index: 0,
        }),

        ValueInner::Map(map) => {
            let keys: Vec<_> = map.keys().cloned().collect();
            Some(ForLoopIterator::Map {
                map: Arc::clone(map),
                keys: keys.into_iter(),
            })
        }

        ValueInner::String(smart_str) => {
            let content = smart_str.clone().into_arc_str();
            Some(ForLoopIterator::create_string_iterator(content))
        }

        ValueInner::Bytes(bytes) => Some(ForLoopIterator::Bytes {
            bytes: Arc::clone(bytes),
            index: 0,
        }),

        _ => None,
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Loop {
    index0: usize,
    first: bool,
    last: bool,
    length: usize,
}

impl Loop {
    /// // 1-based index derived from 0-based
    #[inline(always)]
    fn index(&self) -> usize {
        self.index0 + 1
    }

    #[inline(always)]
    fn advance(&mut self) {
        self.index0 += 1;
        self.first = false;
        self.last = self.index() == self.length;
    }
}

#[derive(Debug)]
pub(crate) struct ForLoop {
    iterator: ForLoopIterator,
    loop_data: Loop,
    pub(crate) end_ip: usize,
    pub(crate) context: HashMap<String, Value>,
    value_name: String,
    key_name: Option<String>,
    current_values: (Option<Value>, Value),
    iterated: bool,
    /// List comprehension are desugared to for loops but we don't expose inner loop.* vars
    is_comprehension: bool,
}

impl ForLoop {
    pub fn new(container: Value) -> Self {
        let iterator =
            create_for_loop_iterator(&container).expect("Should only be called on iterable values");

        let length = iterator.size_hint().1.unwrap_or(0);
        let loop_data = Loop {
            index0: 0,
            first: true,
            last: length == 1,
            length,
        };

        Self {
            iterator,
            loop_data,
            end_ip: 0,
            context: HashMap::new(),
            value_name: String::new(), // Will be set by store_local
            key_name: None,
            current_values: (None, Value::undefined()), // Will be set by first advance()
            iterated: false,
            is_comprehension: false,
        }
    }

    pub fn new_comprehension(container: Value) -> Self {
        let mut for_loop = Self::new(container);
        for_loop.is_comprehension = true;
        for_loop
    }

    pub(crate) fn store_local(&mut self, name: &str) {
        if self.key_name.is_none() && !self.value_name.is_empty() {
            // Second call - this is the key name
            self.key_name = Some(name.to_string());
        } else {
            // First call - this is the value name
            self.value_name = name.to_string();
        }
    }

    /// Advance the counter only after the end ip has been set (eg we start incrementing only from the
    /// second time we see the loop)
    #[inline(always)]
    pub(crate) fn advance(&mut self) {
        if let Some((key, value)) = self.iterator.next() {
            self.current_values = (key, value);
            self.iterated = true;
            if self.end_ip != 0 {
                self.loop_data.advance();
                if !self.context.is_empty() {
                    self.context.clear();
                }
            }
        }
    }

    #[inline(always)]
    pub(crate) fn is_over(&self) -> bool {
        self.iterator.size_hint().0 == 0
    }

    pub(crate) fn iterated(&self) -> bool {
        self.iterated
    }

    pub(crate) fn store(&mut self, name: &str, value: Value) {
        self.context.insert(name.to_string(), value);
    }

    #[inline(always)]
    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        // Special casing the loop variable
        match name {
            "__tera_loop_index" if !self.is_comprehension => {
                Some(Value::from(self.loop_data.index() as u64))
            }
            "__tera_loop_index0" if !self.is_comprehension => {
                Some(Value::from(self.loop_data.index0 as u64))
            }
            "__tera_loop_first" if !self.is_comprehension => {
                Some(Value::from(self.loop_data.first))
            }
            "__tera_loop_last" if !self.is_comprehension => Some(Value::from(self.loop_data.last)),
            "__tera_loop_length" if !self.is_comprehension => {
                Some(Value::from(self.loop_data.length as u64))
            }
            _ => {
                // Check if this is the value variable
                if self.value_name == name {
                    return Some(self.current_values.1.clone());
                }

                // Check if this is the key variable
                if self.key_name.as_deref() == Some(name) {
                    return Some(self.current_values.0.clone().unwrap_or(Value::none()));
                }

                self.context.get(name).cloned()
            }
        }
    }
}
