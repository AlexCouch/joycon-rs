extern crate joycon_rs;

use joycon_rs::joycon::*;

fn main(){
    let manager = JoyconManager::new();
    match manager{
        Ok(m) => {
            let connected_joycons = &m.connected_joycons;
            for mut joycon in connected_joycons{
                joycon.add_input_thread_callback(box |buf|{
                    println!("Input buffer: {:?}", buf);
                })
            }
        },
        Err(e) => panic!("Error: {:?}", e)
    }
}