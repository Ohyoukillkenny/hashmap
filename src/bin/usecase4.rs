use hashmap::dummy::*;

fn main() {
    let timber_resources: HashMap<&str, i32> = [("Norway", 100), ("Denmark", 50), ("Iceland", 10)]
        .iter()
        .cloned()
        .collect();

    for (k, v) in &timber_resources {
        println!("key: {}, val: {}", k, v);
    }
}
