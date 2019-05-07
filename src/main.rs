extern crate joycon_rs;

use joycon_rs::joycon::*;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::{RefCell, Cell};

fn main(){
    match JoyconManager::new(){
        Ok(mut jm) => {
            let res = Arc::try_unwrap(jm);
            match res{
                Ok(mut j) => {
                    if (&j).connected_joycons.len() == 0{
                        return;
                    }
                    loop {
                        &j.connected_joycons[0].add_input_report_handler_cb(Box::new(|buf: &[u8]|{
                            println!("Input report: {:?}", buf);
                        }));
                    }
                },
                Err(e) => panic!("Could not unwrap JoyconManager Rc.")
            }
        },
        Err(e) => return
    };
}