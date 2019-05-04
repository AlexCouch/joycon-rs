extern crate joycon_rs;

use joycon_rs::joycon::*;

fn main(){
    let mut manager = match JoyconManager::new(){
        Ok(jm) => jm,
        Err(_) => return
    };
    manager.search_for_joycons();
    loop {
        let mut buf = [0u8; 20];
        let res = manager.connected_joycons[0].read(&mut buf[..]);
        match res{
            Ok(_) => println!("Input report: {:?}", buf),
            Err(e) => eprintln!("Error: {}", e)
        }
    }
}