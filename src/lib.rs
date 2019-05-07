extern crate hidapi;
extern crate threadgroup;
extern crate core;

/// # Version: 0.0.1 documentation
///
/// This `joycon` module is a module that contains all the functionality of the joycon-api for rust, aka `joycon-rs`.
/// The Joycon-API specification is not complete and is heavily subject to change:
///   * Action requests
///     - An action request is an output report that is wrapped in an object and passed into a queue to be sent to the joycon
///     - An action request is validated by the specified joycon's dedicated output thread
///     - See actions for information on actions
///   * Input Report Handler Callbacks
///     - Callbacks that are passed the input report must be handled on the joycon's dedicated input thread
///     - Callbacks must be registered during initialization of the joycons via the `JoyconManager`.
///   * Input Report Wrappers (IRW)
///     - Input Reports must be wrapped in an object that parses the data into something that can be used
///     - Input Reports are raw data and must be handled internally and sorted into their respective input report wrappers
///     - Input Report Wrappers must implement a report parsing algorithm that takes the input report raw data and converts it into something that can be used by dependents
///   * Joycon Representation Object (JRO)
///     - A JRO is simply just representing a Joycon and must remain minimal in size and carry low overhead
///     - A JRO must be or must have a main thread that spawns child threads
///         > An input thread that handles all the input report handler callbacks and creates the necessary parsed input reports
///         > An output thread that handles all the action requests (see action specification), validates the action requests (see action validation), and sends the output reports
///     - A JRO must have methods for creating rumble patterns (in the form of lambdas/callbacks), reading memory (either as lambdas/callbacks or through a memory reader (see memory reader specification)), and getting battery information
///     - A JRO must also have a HID manager for handling disconnecting/unpairing and HID errors
///     - A JRO must also have a proper error handling or crash reporting system
///         > The error handling or crash reporting system will be either default error handling or crash reporting or registered custom handlers through the JoyconManager
///   * JoyconManager
///     - A JoyconManager must spawn a joycon scanner thread that silently and efficiently (low overhead) scans for newly paired/connected joycons
///         > This thread must also monitor the currently paired/connected joycons for unpairing/disconnecting
///     - A JoyconManager must have a way of enabling shared mode by not exposing the process to dependents
///         > Shared mode should be implemented by having the "connected_joycons" field (currently a `Vec<Joycon>`) be of type `HashMap<Joycon, Joycon>` and "null joycons" should be `DummyJoycon`
///     - A JoyconManager must provide a calibration factory or provide some way of building custom calibrations (does not need to be a "factory")
///     - A JoyconManager must have a crash report or error handling registration
///     - A JoyconManager must provide a rumble factory
///         > The rumble factory can be implemented in any way.
///         > Plans for using JSON for constructing packets before parsed into an action is considered for the default implementation
///         > See rumbling below
///   * Actions
///     - An action is an output report that is wrapped in an object that the API can understand
///         > An action report should be readable by the user and the Joycons by being represented in two ways
///             = The buffered output report in the form of a u8 array sent to the Joycon devices
///             = The preconstructed object containing information on how to build the buffered output report
///         > Actions include, but is not limited to:
///             = Rumbling (see rumble patterns and rumble factory specs)
///             = Setting player number LED
///             = Changing the home button light
///             = Request to read memory (see memory manager)
///             = Request to write memory (such as writing user custom calibrations; see memory manager)
///             = Retrieve the current battery voltage and parse into a `BatteryInformation` property
///   * Rumbling
///     - A rumble is a vibration frequency (low and high) and duration (see simple rumble packet below) sent to the Joycon device
///     - A rumble must be sent in the form of a rumble packet, which can be either a simple packet or a complex packet
///         - A simple rumble packet contains a high frequency, low frequency, duration, and device target
///         - A complex rumble packet should be a collection of simple rumble packets that form a rumble pattern
///     - A rumble packet can either be for a single device or several devices
///   * Memory manager
///     - A memory manager must be present within a JRO for safe, secure, and efficient memory reading/writing
///
///

pub mod joycon{
    use hidapi::*;
    use std::thread;
    use threadgroup::ThreadGroup;
    use std::sync::{Mutex, Arc};
    use std::rc::{Weak, Rc};
    use core::borrow::{Borrow, BorrowMut};
    use std::thread::JoinHandle;
    use std::fmt::{Error, Formatter};
    use std::os::raw::c_void;

    const JC_VENDOR_ID: u16 = 1406;
    const LEFT_JC_PROD_ID: u16 = 8198;
    const RIGHT_JC_PROD_ID: u16 = 8199;

    trait Threaded {
        fn start(&self) -> JoinHandle<()>;
    }

    struct InputReportHandlerThread{
        input_report_handlers: Vec<Box<dyn Fn(&[u8]) -> ()>>,
        input_device: Arc<HidDevice>
    }

    unsafe impl Send for c_void{}

    impl Threaded for InputReportHandlerThread{
        fn start(&self) -> JoinHandle<()>{
             return thread::spawn(move||{
                 let dev = Arc::clone(&self.input_device);
                 loop{
                     let mut buf: &[u8] = &[];
                     dev.read(&mut buf).unwrap();
                     for handler in self.input_report_handlers{
                         handler(buf)
                     }
                 }
            })
        }
    }

    #[derive(Debug)]
    pub struct Joycon {
        handle: Arc<HidDeviceInfo>,
        input_thread_join_handle: InputReportHandlerThread,
        output_thread_join_handle: JoinHandle<Option<bool>>,
    }

    #[derive(Clone)]
    pub struct JoyconProperties{
        input_report_handlers: Vec<fn(buf: &[u8])>,
    }

    impl Joycon{
        pub fn new(mut manager: Rc<JoyconManager>, handle: HidDeviceInfo) -> Option<Joycon>{
            let mut async_manager = JoyconManager::get_async(Rc::clone(&manager));
            let mut async_handle = Arc::new(handle);
            let device = Arc::new(manager.hidapi.open(async_handle.vendor_id, async_handle.product_id).unwrap());
            let jc = Rc::new(Joycon {
                handle: Arc::clone(&async_handle),
                input_thread_join_handle: InputReportHandlerThread{
                    input_report_handlers: vec![],
                    input_device: Arc::clone(&device)
                },
                output_thread_join_handle: thread::spawn::<_,Option<bool>>(move || { Some(true) })
            });
            jc.input_thread_join_handle.start();
            return Ok(Rc::try_unwrap(jc).unwrap());
        }

        pub fn add_input_report_handler_cb(&mut self, callback: fn(buf: &[u8]) -> bool){
            self.input_thread_join_handle.input_report_handlers.push(Box::new( callback));
        }
    }

    pub type JoyconResult = Result<Joycon, Error>;

    #[derive(Debug)]
    pub struct JoyconManager{
        hidapi: HidApi,
        pub connected_joycons: Vec<Joycon>
    }

    impl JoyconManager{
        pub fn new() -> std::result::Result<Rc<JoyconManager>, HidError>{
            let _hidapi = match HidApi::new(){
                Ok(h) => h,
                Err(e) => return Err(e)
            };

            let mut jm = Rc::new(JoyconManager{
                hidapi: _hidapi,
                connected_joycons: Vec::new()
            });
            JoyconManager::search_for_joycons(Rc::clone(&jm));
            Ok(Rc::clone(&jm))
        }

        pub fn get_async(mut this: Rc<JoyconManager>) -> Arc<JoyconManager>{
            let jm = Rc::try_unwrap(this)?;
            Arc::new(jm)
        }

        /// Searches for joycons and pushes them to `connected_joycons`
        /// Can be used for refreshing the list of joycons, however, this is called internally by a refresh thread
        /// And called upon JoyconManager initialization
        pub fn search_for_joycons(mut this: Rc<JoyconManager>){
            let devs = this.hidapi.devices();
            for _d in devs.iter(){
                let d = _d.clone();
                let vendor_id = d.vendor_id;
                let prod_id = d.product_id;
                //Cleanup syntax? I dunno, trying to make it more readable but this feels weird tbh
                if vendor_id == JC_VENDOR_ID &&
                    (
                        (prod_id == LEFT_JC_PROD_ID) ||
                        (prod_id == RIGHT_JC_PROD_ID)
                ) {
                    // TODO: Better error handling
                    let jc = Joycon::new(Rc::clone(&this), d);
                    match jc {
                        Ok(dev) => {
                            this.connected_joycons.push(dev);
                            true
                        },
                        Err(_) => {
                            eprintln!("Error: Could not initialize Joycon."); // @TODO: ERRImprove
                            false
                        }
                    };
                }
            }
        }
    }
}
