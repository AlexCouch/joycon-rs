extern crate joycon_rs;

use joycon_rs::*;
use joycon_rs::joycon::JoyconManager;

fn main(){
    let mut manager = match JoyconManager::new(){
        Ok(jm) => jm,
        Err(e) => return
    };
    manager.search_for_joycons();
    for d in manager.connected_joycons {
        assert_eq!(d.product_string.is_some(), true);
        println!("{}", d.product_string.unwrap());
    }
}