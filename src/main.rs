use kvstore::KvStore;

fn main() -> kvstore::error::Result<()> {
    
    let mut store = KvStore::open("./data")?;
    store.set("user".into(), "Aswin".into())?;
    println!("{:?}", store.get("user".into())?);
    store.remove("user".into())?;
    println!("{:?}", store.get("user".into())?);
    Ok(())
}
