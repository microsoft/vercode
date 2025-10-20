// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{Vercode, deserialize, serialize};

#[test]
fn hashmap_basic() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert(1u32, "one".to_string());
    map.insert(2u32, "two".to_string());
    map.insert(3u32, "three".to_string());

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&map, &mut buf);
    let decoded: HashMap<u32, String> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded.get(&1), Some(&"one".to_string()));
    assert_eq!(decoded.get(&2), Some(&"two".to_string()));
    assert_eq!(decoded.get(&3), Some(&"three".to_string()));
}

#[test]
fn hashmap_empty() {
    use std::collections::HashMap;

    let map: HashMap<u32, String> = HashMap::new();

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&map, &mut buf);
    let decoded: HashMap<u32, String> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 0);
    assert_eq!(serialized.len(), 4); // Just the length prefix
}

#[test]
fn hashmap_nested_values() {
    use std::collections::HashMap;

    #[derive(Vercode, Debug, PartialEq, Clone)]
    struct Person {
        name: String,
        age: u32,
    }

    let mut map = HashMap::new();
    map.insert(
        1,
        Person {
            name: "Alice".to_string(),
            age: 30,
        },
    );
    map.insert(
        2,
        Person {
            name: "Bob".to_string(),
            age: 25,
        },
    );

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&map, &mut buf);
    let decoded: HashMap<u32, Person> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded.get(&1).unwrap().name, "Alice");
    assert_eq!(decoded.get(&2).unwrap().age, 25);
}

#[test]
fn hashset_basic() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(1u32);
    set.insert(2u32);
    set.insert(3u32);
    set.insert(4u32);
    set.insert(5u32);

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&set, &mut buf);
    let decoded: HashSet<u32> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 5);
    assert!(decoded.contains(&1));
    assert!(decoded.contains(&2));
    assert!(decoded.contains(&3));
    assert!(decoded.contains(&4));
    assert!(decoded.contains(&5));
}

#[test]
fn hashset_empty() {
    use std::collections::HashSet;

    let set: HashSet<u32> = HashSet::new();

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&set, &mut buf);
    let decoded: HashSet<u32> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 0);
    assert_eq!(serialized.len(), 4); // Just the length prefix
}

#[test]
fn hashset_strings() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert("apple".to_string());
    set.insert("banana".to_string());
    set.insert("cherry".to_string());

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&set, &mut buf);
    let decoded: HashSet<String> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 3);
    assert!(decoded.contains("apple"));
    assert!(decoded.contains("banana"));
    assert!(decoded.contains("cherry"));
}

#[test]
fn hashmap_in_struct() {
    use std::collections::HashMap;

    #[derive(Vercode, Debug, PartialEq, Clone)]
    struct Container {
        data: HashMap<String, u32>,
        name: String,
    }

    let mut data = HashMap::new();
    data.insert("count".to_string(), 42);
    data.insert("size".to_string(), 100);

    let container = Container {
        data,
        name: "test".to_string(),
    };

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&container, &mut buf);
    let decoded: Container = deserialize(serialized).unwrap();

    assert_eq!(decoded, container);
    assert_eq!(decoded.data.get("count"), Some(&42));
    assert_eq!(decoded.data.get("size"), Some(&100));
}

#[test]
fn hashmap_nested() {
    use std::collections::HashMap;

    // HashMap<String, HashMap<String, u32>>
    let mut inner1 = HashMap::new();
    inner1.insert("a".to_string(), 1u32);
    inner1.insert("b".to_string(), 2u32);

    let mut inner2 = HashMap::new();
    inner2.insert("x".to_string(), 10u32);
    inner2.insert("y".to_string(), 20u32);
    inner2.insert("z".to_string(), 30u32);

    let mut outer = HashMap::new();
    outer.insert("first".to_string(), inner1);
    outer.insert("second".to_string(), inner2);

    let mut buf = vec![0u8; 2048];
    let serialized = serialize(&outer, &mut buf);
    let decoded: HashMap<String, HashMap<String, u32>> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 2);

    let first = decoded.get("first").unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first.get("a"), Some(&1));
    assert_eq!(first.get("b"), Some(&2));

    let second = decoded.get("second").unwrap();
    assert_eq!(second.len(), 3);
    assert_eq!(second.get("x"), Some(&10));
    assert_eq!(second.get("y"), Some(&20));
    assert_eq!(second.get("z"), Some(&30));
}

#[test]
fn btreemap_basic() {
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    map.insert(1u32, "one".to_string());
    map.insert(2u32, "two".to_string());
    map.insert(3u32, "three".to_string());

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&map, &mut buf);
    let decoded: BTreeMap<u32, String> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded.get(&1), Some(&"one".to_string()));
    assert_eq!(decoded.get(&2), Some(&"two".to_string()));
    assert_eq!(decoded.get(&3), Some(&"three".to_string()));
}

#[test]
fn btreemap_empty() {
    use std::collections::BTreeMap;

    let map: BTreeMap<u32, String> = BTreeMap::new();

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&map, &mut buf);
    let decoded: BTreeMap<u32, String> = deserialize(serialized).unwrap();

    assert_eq!(decoded.len(), 0);
    assert_eq!(serialized.len(), 4); // Just the length prefix
}

#[test]
fn btreemap_ordering() {
    use std::collections::BTreeMap;

    // BTreeMap maintains sorted order
    let mut map = BTreeMap::new();
    map.insert(3u32, "three".to_string());
    map.insert(1u32, "one".to_string());
    map.insert(2u32, "two".to_string());

    let mut buf = vec![0u8; 1024];
    let serialized = serialize(&map, &mut buf);
    let decoded: BTreeMap<u32, String> = deserialize(serialized).unwrap();

    // Check that iteration order is maintained (sorted)
    let keys: Vec<_> = decoded.keys().copied().collect();
    assert_eq!(keys, vec![1, 2, 3]);
}
