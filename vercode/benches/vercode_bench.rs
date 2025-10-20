// Copyright (c) Microsoft Corporation. All rights reserved.
use std::{collections::HashMap, num::NonZeroU64, time::Duration};
use uuid::Uuid;

const EXAMPLE: E = E {
    id: Uuid::nil(),
    r: R::W {
        l: 1,
        o: 2,
        f: 3,
        t: 4,
        c: [
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
            C(0),
        ],
        q: Q {
            p: P(5),
            w: W { a: 6, b: 7, c: 8 },
            c: Some(S(NonZeroU64::new(9).unwrap())),
            s: S(NonZeroU64::new(10).unwrap()),
        },
    },
};

#[derive(Copy, Clone, vercode::Vercode, serde::Serialize, serde::Deserialize)]
struct E {
    id: Uuid,
    r: R,
}

#[derive(Copy, Clone, vercode::Vercode, serde::Serialize, serde::Deserialize)]
enum R {
    Empty,
    W {
        l: usize,
        o: usize,
        f: u128,
        t: u64,
        c: [C; 16],
        q: Q,
    },
    C {
        a: TS,
        b: u128,
        c: CS,
        q: Q,
    },
    L {
        a: TS,
        b: TS,
        c: bool,
        d: Q,
    },
    D {
        a: TS,
        q: Q,
    },
    G,
}

#[derive(Copy, Clone, vercode::Vercode, serde::Serialize, serde::Deserialize)]
struct Q {
    p: P,
    w: W,
    c: Option<S>,
    s: S,
}

#[derive(Copy, Clone, vercode::Vercode, serde::Serialize, serde::Deserialize)]
struct W {
    a: i64,
    b: i64,
    c: i64,
}

#[derive(Copy, Clone, vercode::Vercode, serde::Serialize, serde::Deserialize)]
struct S(NonZeroU64);

#[derive(Copy, Clone, vercode::Vercode, serde::Serialize, serde::Deserialize)]
struct P(u64);

#[derive(
    Copy, Clone, Default, vercode::VercodeTransparent, serde::Serialize, serde::Deserialize,
)]
struct C(u32);

#[derive(Copy, Clone, Debug, vercode::Vercode, serde::Serialize, serde::Deserialize)]
struct CS {
    a: usize,
    b: u64,
    c: u32,
    d: u32,
}

pub enum T {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
}

#[derive(Copy, Clone, Debug, vercode::Vercode, serde::Serialize, serde::Deserialize)]
enum Choice {
    Yes,
    No,
}

#[derive(Copy, Clone, Debug, vercode::Vercode, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
enum TS {
    One {
        a: u32,
        b: u64,
    } = T::One as u8,
    Two {
        a: u32,
    } = T::Two as u8,
    Three {
        a: u32,
    } = T::Three as u8,
    Four {
        a: u32,
        b: u64,
    } = T::Four as u8,
    Five {
        a: u32,
        b: u64,
        c: u32,
    } = T::Five as u8,
    Six {
        choice: Choice,
        a: u32,
        b: u64,
    } = T::Six as u8,
    Seven {
        choice: Choice,
        a: u32,
    } = T::Seven as u8,
    Eight {
        a: u32,
        b: u64,
        c: u32,
        choice: Choice,
    } = T::Eight as u8,
    Nine {
        choice: Choice,
        a: u32,
        b: u64,
    } = T::Nine as u8,
}

fn get_example() -> &'static E {
    &EXAMPLE
}

pub fn benchmark(c: &mut criterion::Criterion) {
    c.bench_function("vercode_serialize_vec_u8", |b| {
        let buffer = vec![42u32; 512];

        b.iter(|| {
            let mut local = [0u8; 4096];
            let _encoded = std::hint::black_box(vercode::serialize(&buffer, &mut local));
        });
    });

    c.bench_function("vercode_deserialize_vec_u8", |b| {
        let buffer = vec![42u32; 512];
        let mut local = [0u8; 4096];
        let encoded = std::hint::black_box(vercode::serialize(&buffer, &mut local));

        b.iter(|| {
            let _result = std::hint::black_box(vercode::deserialize::<Vec<u32>>(encoded));
        });
    });

    c.bench_function("example_vercode_serialize", |b| {
        b.iter(|| {
            let mut local = [0u8; 256];
            std::hint::black_box(vercode::serialize(
                std::hint::black_box(get_example()),
                &mut local,
            ));
        });
    });

    c.bench_function("example_vercode_deserialize", |b| {
        let mut local = [0u8; 256];
        let local = std::hint::black_box(vercode::serialize(&EXAMPLE, &mut local));

        b.iter(|| {
            let _result = std::hint::black_box(vercode::deserialize::<E>(local));
        });
    });

    c.bench_function("example_bincode_serialize", |b| {
        b.iter(|| {
            let mut local = [0u8; 256];
            let mut local_mut = &mut local[..];
            let result = std::hint::black_box(bincode::serialize_into(&mut local_mut, &EXAMPLE));
            result.unwrap();
        });
    });

    c.bench_function("example_bincode_deserialize", |b| {
        let mut local = [0u8; 256];
        let mut local_mut = &mut local[..];
        let result = std::hint::black_box(bincode::serialize_into(&mut local_mut, &EXAMPLE));
        result.unwrap();
        let length = 256 - local_mut.len();
        let encoded = &local[..length];

        b.iter(|| {
            let result = std::hint::black_box(bincode::deserialize::<E>(encoded));
            result.unwrap();
        });
    });

    // HashMap<u32, u32> benchmarks
    let hashmap: HashMap<u32, u32> = (0..100).map(|i| (i, i * 2)).collect();

    c.bench_function("hashmap_vercode_roundtrip", |b| {
        b.iter(|| {
            let mut local = [0u8; 2048];
            let encoded = std::hint::black_box(vercode::serialize(&hashmap, &mut local));
            let _result = std::hint::black_box(vercode::deserialize::<HashMap<u32, u32>>(encoded));
        });
    });

    c.bench_function("hashmap_bincode_roundtrip", |b| {
        b.iter(|| {
            let mut local = [0u8; 2048];
            let mut local_mut = &mut local[..];
            let result = std::hint::black_box(bincode::serialize_into(&mut local_mut, &hashmap));
            result.unwrap();
            let length = 2048 - local_mut.len();
            let encoded = &local[..length];
            let result = std::hint::black_box(bincode::deserialize::<HashMap<u32, u32>>(encoded));
            result.unwrap();
        });
    });
}

criterion::criterion_group!(
    name = benches;
    config = criterion::Criterion::default().warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(5));
    targets=benchmark
);
criterion::criterion_main!(benches);
