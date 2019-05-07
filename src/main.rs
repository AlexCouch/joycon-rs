extern crate joycon_rs;

use joycon_rs::joycon::*;
use std::sync::{Arc, Mutex};
use std::rc::Rc;

fn main(){
    let mut manager = match JoyconManager::new(){
        Ok(mut jm) => jm,
        Err(e) => return
    };
    loop {
        manager.connected_joycons[0].add_input_report_handler_cb(move |buf|{
            println!("Input report: {:?}", buf);
            true
        });
    }
}