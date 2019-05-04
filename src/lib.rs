extern crate hidapi;
extern crate threadgroup;
extern crate core;

pub mod joycon{
    use hidapi::*;
    use std::thread;
    use threadgroup::{JoinError, ThreadGroup};

    const JC_VENDOR_ID: u16 = 1406;
    const LEFT_JC_PROD_ID: u16 = 8198;
    const RIGHT_JC_PROD_ID: u16 = 8199;

    pub struct Joycon<'a> {
        handle: &'a HidDevice, //Tried storing a reference with a lifetime of the Joycon object
        properties: JoyconProperties,
        thread_group: ThreadGroup<bool>
    }

    pub struct JoyconProperties{
        input_report_handlers: Vec<fn(buf: &[u8])>,
    }

    impl<'a> Joycon<'a>{

        //Tried having a reference to the HidDevice passed in and then stored into the Joycon object
        // Also tried having a separate properties object passed into the thread but then realized I need access to everything in the Joycon
        pub fn new(handle: &HidDevice) -> std::result::Result<Joycon, JoinError>{
            let mut thread_group = ThreadGroup::<bool>::new();
            let mut properties = JoyconProperties{
                input_report_handlers: Vec::new()
            };
            let jc = Joycon{
                handle,
                properties,
                thread_group
            };

            // Spawn a thread and create local captures of the handle and properties
            // (but can probably just delegate to a single copy of the Joycon and get the handle and properties that way)
            let input_handler_thread = thread::spawn::<_,bool>(move ||{
                let mut ret = true;
                let h = handle; //Causes a crash because of the HidDevice field of type `hidapi::ffi::HidDevice` which is a `pub type HidDevice = *mut c_void`
                let jprops = &properties;
                let mut buf = [0u8; 50];
                loop{
                    let res = h.read(&mut buf);
                    match res {
                        Ok(_) => {
                            for handler in jprops.input_report_handlers {
                                println!("[Input Handler Thread]: Handling input...");
                                handler(&mut buf);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            ret = false;
                            break
                        }
                    }
                }
                ret
            });
            Ok(jc)
        }

        pub fn add_input_report_handler_cb(&mut self, callback: fn(buf: &[u8])){
            self.properties.input_report_handlers.push(callback);
        }
    }

    pub struct JoyconManager{
        hidapi: HidApi,

        pub connected_joycons: Vec<HidDevice>
    }

    impl JoyconManager{

        pub fn new() -> std::result::Result<JoyconManager, HidError>{
            let _hidapi = match HidApi::new(){
                Ok(h) => h,
                Err(e) => return Err(e)
            };
            let mut jm = JoyconManager{
                hidapi: _hidapi,
                connected_joycons: Vec::new()
            };
            jm.search_for_joycons();
            Ok(jm)
        }

        /// Searches for joycons and pushes them to `connected_joycons`
        /// Can be used for refreshing the list of joycons, however, this is called internally by a refresh thread
        /// And called upon JoyconManager initialization
        pub fn search_for_joycons(&mut self){
            let devs = self.hidapi.devices();
            for _d in devs.iter(){
                let d = _d.clone();
                let vendor_id = d.vendor_id;
                let prod_id = d.product_id;
                //Cleanup syntax? I dunno, trying to make it more readable but this feels weird tbh
                if vendor_id == JC_VENDOR_ID &&
                    (
                        (prod_id == LEFT_JC_PROD_ID) ||
                        (prod_id == RIGHT_JC_PROD_ID)
                    )
                {
                    let _dev = self.hidapi.open(d.vendor_id, d.product_id);
                    match _dev {
                        Ok(dev) => self.connected_joycons.push(dev),
                        Err(e) => eprintln!("Error: {}", e)
                    }
                }
            }
        }
    }
}
