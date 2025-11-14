pub trait OptionStringExt {
    fn clean(self) -> Option<String>;
}

impl OptionStringExt for Option<String> {
    fn clean(self) -> Option<String> {
        self.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }
}

pub trait StringExt {
    fn clean(self) -> Option<String>;
}

impl StringExt for String {
    fn clean(self) -> Option<String> {
        let trimmed = self.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

use std::collections::BTreeMap;

pub trait BTreeMapExt<K, V> {
    fn get_cloned<T, F>(&self, key: &K, f: F) -> Option<T>
    where
        F: Fn(&V) -> &T,
        T: Clone;
}

impl<K, V> BTreeMapExt<K, V> for BTreeMap<K, V>
where
    K: Ord,
{
    fn get_cloned<T, F>(&self, key: &K, f: F) -> Option<T>
    where
        F: Fn(&V) -> &T,
        T: Clone,
    {
        self.get(key).map(|value| f(value).clone())
    }
}
