fn main() -> Result<(), std::io::Error> {
    println!("hello wal");
    let result: u64 = wal::add(1, 3);
    println!("result = {}", result);
    Ok(())
}
