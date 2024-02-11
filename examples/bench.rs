#[macro_use]
extern crate bencher;

use bencher::{black_box, Bencher};
use jsurl::serialize;

fn bench_serialize(b: &mut Bencher) {
    let obj: serde_json::Value =
        serde_json::from_str(r#"{"name":"John Doe","age":42,"children":["Mary","Bill"]}"#).unwrap();
    b.iter(|| {
        let s = serialize(&obj);
        black_box(s);
    });
}

benchmark_group!(benches, bench_serialize);
benchmark_main!(benches);
