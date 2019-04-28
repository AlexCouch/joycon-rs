extern crate hidapi;

pub mod joycon{
    use hidapi::*;

    const JC_VENDOR_ID: u16 = 1406;
    const LEFT_JC_PROD_ID: u16 = 8198;
    const RIGHT_JC_PROD_ID: u16 = 8199;

    pub struct JoyconManager{
        hidapi: HidApi,

        pub connected_joycons: Vec<HidDeviceInfo>
    }

    impl JoyconManager{

        pub fn new() -> Result<JoyconManager, HidError>{
            let _hidapi = match HidApi::new(){
                Ok(h) => h,
                Err(e) => return Err(e)
            };
            Ok(
                JoyconManager{
                    hidapi: _hidapi,
                    connected_joycons: Vec::new()
                }
            )
        }

        /// Searches for joycons and pushes them to `connected_joycons`
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
                    self.connected_joycons.push(d);
                }
            }
        }
    }
}
