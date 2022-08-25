use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem;

const INITIAL_BUCKET_SIZE: usize = 1;

pub struct HashMap<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    num_items: usize,
}

impl<K, V> HashMap<K, V> {
    pub fn new() -> Self {
        HashMap {
            buckets: vec![],
            num_items: 0,
        }
    }
}

// now implement entry for in-place modification

pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    entry: &'a mut (K, V), // address of a kv pair
}

pub struct VacantEntry<'a, K: 'a, V: 'a> {
    key: K,
    map: &'a mut HashMap<K, V>,
    bucket_index: usize,
}

impl <'a, K, V> VacantEntry<'a, K, V>
where K: Hash + Eq
{
    pub fn insert(self, val: V) -> &'a mut V {
        self.map.buckets[self.bucket_index].push((self.key, val));
        self.map.num_items += 1;
        &mut self.map.buckets[self.bucket_index].last_mut().unwrap().1
    }
}

pub enum Entry<'a, K: 'a, V: 'a> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl <'a, K, V> Entry<'a, K, V>
where K: Hash + Eq
{
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => &mut e.entry.1,
            Entry::Vacant(e) => e.insert(default),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut V
    where F: FnOnce() -> V,
    {
        match self {
            Entry::Occupied(e) => &mut e.entry.1,
            Entry::Vacant(e) => e.insert(default()),
        }
    }
}

impl<K, V> HashMap<K, V>
where
    K: Hash + Eq,
{
    fn get_bucket_index<Q>(&mut self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.buckets.is_empty() {
            return None;
        }
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        Some((hasher.finish() % self.buckets.len() as u64) as usize)
    }

    fn resize(&mut self) {
        let capacity = match self.num_items {
            0 => INITIAL_BUCKET_SIZE,
            n => 2 * n,
        };
        // resize the bucket by rehash all keys
        let mut new_buckets = Vec::with_capacity(capacity);
        new_buckets.extend((0..capacity).map(|_| vec![]));

        for (key, val) in self.buckets.iter_mut().flat_map(|b| b.drain(..)) {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            let bucket_index = (hasher.finish() % capacity as u64) as usize;
            new_buckets[bucket_index].push((key, val));
        }
        let _ = mem::replace(&mut self.buckets, new_buckets);
    }

    pub fn is_empty(&self) -> bool {
        self.num_items == 0
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Hash + Eq,
    {
        if self.buckets.is_empty() || self.num_items > 3 * self.buckets.len() / 4 {
            self.resize();
        }

        // hash the key
        let bucket_index = self
            .get_bucket_index(&key)
            .expect("bucket is empty handled above");
        let bucket = &mut self.buckets[bucket_index];
        for &mut (ref ekey, ref mut eval) in bucket.iter_mut() {
            if ekey == &key {
                return Some(mem::replace(eval, value));
            }
        }
        self.num_items += 1;
        bucket.push((key, value));
        None
    }

    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let bucket_index = self.get_bucket_index(key)?;
        self.buckets[bucket_index]
            .iter()
            .find(|&(ref ekey, _)| ekey.borrow() == key)
            .map(|&(_, ref v)| v)
    }

    pub fn contains_key<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.num_items
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let bucket_index = self.get_bucket_index(key)?;
        let bucket = &mut self.buckets[bucket_index];
        let i = bucket
            .iter()
            .position(|&(ref ekey, _)| ekey.borrow() == key)?;
        self.num_items -= 1;
        Some(bucket.swap_remove(i).1)
    }

    pub fn entry<'a>(&'a mut self, key: K) -> Entry<'a, K, V> {
        if self.buckets.is_empty() || self.num_items > 3 * self.buckets.len() / 4 {
            self.resize();
        }
        // may consider entry as adding a new key-val pair to the hashmap
        let bucket_index = self
            .get_bucket_index(&key)
            .expect("bucket is empty handled above");

        match self.buckets[bucket_index].iter().position(|&(ref ekey, _)| ekey == &key) {
            Some(at) => Entry::Occupied(OccupiedEntry{
                entry: &mut self.buckets[bucket_index][at],
            }),
            None => Entry::Vacant(VacantEntry{
                key,
                map: self,
                bucket_index
            })
        }
    }

}

// implement the hashmap as an iterator

// we first implement the iter of key and val with reference
pub struct RefIter<'a, K: 'a, V: 'a> {
    map: &'a HashMap<K, V>,
    bucket_index: usize,
    at: usize,
}

impl<'a, K, V> Iterator for RefIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.map.buckets.get(self.bucket_index) {
                Some(bucket) => match bucket.get(self.at) {
                    Some(&(ref k, ref v)) => {
                        self.at += 1;
                        break Some((k, v));
                    }
                    None => {
                        self.bucket_index += 1;
                        self.at = 0;
                        continue;
                    }
                },
                None => break None,
            }
        }
    }
}

impl<'a, K, V> IntoIterator for &'a HashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = RefIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        RefIter {
            map: self,
            bucket_index: 0,
            at: 0,
        }
    }
}

pub struct ItemIter<K, V> {
    map: HashMap<K, V>,
    bucket_index: usize,
}

impl<K, V> Iterator for ItemIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.map.buckets.get_mut(self.bucket_index) {
                Some(bucket) => match bucket.pop() {
                    Some(x) => {
                        break Some(x);
                    }
                    None => {
                        self.bucket_index += 1;
                        continue;
                    }
                },
                None => break None,
            }
        }
    }
}

impl<K, V> IntoIterator for HashMap<K, V> {
    type Item = (K, V);
    type IntoIter = ItemIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        ItemIter {
            map: self,
            bucket_index: 0,
        }
    }
}

// implement FromIterator so that we can collect items
impl<K, V> FromIterator<(K, V)> for HashMap<K, V>
where K: Hash + Eq,
{
    fn from_iter<T>(iter: T) -> Self
    where T: IntoIterator<Item=(K, V)>,
    {
        let mut map = HashMap::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
        let mut map = HashMap::new();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        map.insert("foo", 42);
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());
        assert_eq!(map.get(&"foo"), Some(&42));
        assert_eq!(map.remove(&"foo"), Some(42));
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert_eq!(map.get(&"foo"), None);
    }

    #[test]
    fn iter() {
        let mut map = HashMap::new();
        map.insert("foo", 42);
        map.insert("bar", 43);
        map.insert("baz", 142);
        map.insert("quox", 7);
        for (&k, &v) in &map {
            match k {
                "foo" => assert_eq!(v, 42),
                "bar" => assert_eq!(v, 43),
                "baz" => assert_eq!(v, 142),
                "quox" => assert_eq!(v, 7),
                _ => unreachable!(),
            }
        }
        assert_eq!((&map).into_iter().count(), 4);

        let mut items = 0;
        for (k, v) in map {
            match k {
                "foo" => assert_eq!(v, 42),
                "bar" => assert_eq!(v, 43),
                "baz" => assert_eq!(v, 142),
                "quox" => assert_eq!(v, 7),
                _ => unreachable!(),
            }
            items += 1;
        }
        assert_eq!(items, 4);
    }

    #[test]
    fn empty_hashmap() {
        let mut map = HashMap::<&str, &str>::new();
        assert_eq!(map.contains_key("k"), false);
        assert_eq!(map.get("k"), None);
        assert_eq!(map.remove("k"), None);
    }
}
