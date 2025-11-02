use kvstore::KvStore;

fn main() -> kvstore::error::Result<()> {
    println!("Opening store...");
    let mut store = KvStore::open("./data")?;
    println!("Store opened successfully!");
    
    println!("Setting key 'user' to 'Aswin'...");
    store.set("user".into(), "Aswin".into())?;
    println!("Key set successfully!");
    
    println!("Getting key 'user'...");
    println!("{:?}", store.get("user".into())?);
    
    println!("Removing key 'user'...");
    store.remove("user".into())?;
    println!("Key removed successfully!");
    
    println!("Getting key 'user' again...");
    println!("{:?}", store.get("user".into())?);
    Ok(())
}
