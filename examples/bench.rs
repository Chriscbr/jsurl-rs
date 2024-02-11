#[macro_use]
extern crate bencher;

use bencher::{black_box, Bencher};
use jsurl::{deserialize, serialize};

fn bench_serialize(b: &mut Bencher) {
    let obj: serde_json::Value =
        serde_json::from_str(r#"{"name":"John Doe","age":42,"children":["Mary","Bill"]}"#).unwrap();
    b.iter(|| {
        let s = serialize(&obj);
        black_box(s);
    });
}

fn bench_deserialize(b: &mut Bencher) {
    let s = r#"~(name~'John*20Doe~age~42~children~(~'Mary~'Bill))"#;
    b.iter(|| {
        let s = deserialize(s).unwrap();
        black_box(s);
    });
}

benchmark_group!(benches, bench_serialize, bench_deserialize);
benchmark_main!(benches);
