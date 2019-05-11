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
///

pub mod joycon{
    use hidapi::*;
    use std::thread;
    use std::thread::JoinHandle;
    use std::sync::mpsc::{Sender, Receiver, channel};
    use std::sync::Arc;

    const JC_VENDOR_ID: u16 = 1406;
    const LEFT_JC_PROD_ID: u16 = 8198;
    const RIGHT_JC_PROD_ID: u16 = 8199;

    // ==================== Joycon =================================

    pub struct Joycon {
        device_handle: Arc<HidDeviceInfo>,
        input_handler_thread: InputHandlerThread
    }

    pub struct InputHandlerThread{
        join_handle: Option<JoinHandle<()>>,
        input_handler_callbacks: Vec<Box<Fn(Vec<u8>) + 'static + Send + Sync>>,
        callback_channel: CallbackChannel
    }

    struct DeviceChannel(Sender<Arc<HidDeviceInfo>>, Receiver<Arc<HidDeviceInfo>>);
    struct CallbackChannel(Sender<Vec<Box<Fn(Vec<u8>) + Send + Sync + 'static>>>, Receiver<Vec<Box<Fn(Vec<u8>) + Send + Sync + 'static>>>);

    impl InputHandlerThread{
        pub fn new(device: Arc<HidDeviceInfo>, manager: Arc<JoyconManager>) -> Self{
            let (dev_sender, dev_receiver) = channel::<Arc<HidDeviceInfo>>();
            let (cb_sender, cb_receiver) = channel::<Vec<Box<Fn(Vec<u8>) + Send + Sync + 'static>>>();
            let mut input_handler_thread = Self{
                join_handle: None,
                input_handler_callbacks: Vec::new(),
                callback_channel: CallbackChannel(cb_sender, cb_receiver)
            };
            &input_handler_thread.start(device, manager);
            return input_handler_thread;
        }

        fn start(&mut self, device: Arc<HidDeviceInfo>, manager: Arc<JoyconManager>){
            let &cb_sender = &self.callback_channel.0;
            let &cb_receiver = &self.callback_channel.1;
            let &callbacks = &self.input_handler_callbacks;
            cb_sender.send(callbacks);
            self.join_handle = Some(thread::spawn(move ||{
                let handle = manager.hidapi.open(device.vendor_id, device.product_id).unwrap();
                let mut buf = [0u8];
                let cb_vec = cb_receiver.recv().unwrap();
                handle.read(&mut buf);
                for cb in cb_vec{
                    cb(buf.to_vec())
                }
            }));
        }
    }

    impl Joycon {
        pub fn new(device_info: HidDeviceInfo, manager: Arc<JoyconManager>) -> JoyconResult<Joycon>{
            let device_arc = Arc::new(device_info);
            let input_handler_thread = InputHandlerThread::new(Arc::clone(&device_arc), Arc::clone(manager));
            let joycon = Joycon{
                device_handle: Arc::clone(&device_arc),
                input_handler_thread
            };
            return Ok(joycon)
        }

        pub fn add_input_thread_callback<F>(&mut self, cb: F) where F : Fn(Vec<u8>) + Send + Sync + 'static{
            self.input_handler_thread.input_handler_callbacks.push(Box::new(cb))
        }
    }

    // ================= JoyconManager ==============================

    pub struct JoyconManager{
        hidapi: HidApi,
        pub connected_joycons: Vec<Joycon>,
    }

    pub type JoyconResult<T> = Result<T, JoyconError>;

    #[derive(Debug)]
    pub enum JoyconError{
        InitializationError(String)
    }

    impl JoyconManager{
        //Much cleaner than before
        pub fn new() -> JoyconResult<JoyconManager>{
            let hidapi = HidApi::new();
            match hidapi{
                Ok(h) => {
                    let mut manager = JoyconManager{
                        hidapi: h,
                        connected_joycons: vec![]
                    };
                    manager.search_for_joycons();
                    return Ok(manager);
                },
                Err(e) => panic!("Error: {}", e)
            };
            return Err(JoyconError::InitializationError("Could not initialize JoyconManager.".to_string()))
        }

        pub fn search_for_joycons(mut self){
            let arc = Arc::new(self);
            for d in arc.hidapi.devices(){
                let vendor_id = d.vendor_id.clone();
                let product_id = d.product_id.clone();
                if vendor_id == JC_VENDOR_ID && (product_id == RIGHT_JC_PROD_ID || product_id == LEFT_JC_PROD_ID){
                    let joycon = Joycon::new(d, Arc::clone(&arc)).unwrap();
                    arc.connected_joycons.push(joycon)
                }
            }
        }
    }
}

/*pub mod joycon{
    use hidapi::*;
    use std::thread;
    use threadgroup::ThreadGroup;
    use std::sync::{Mutex, Arc};
    use std::rc::{Weak, Rc};
    use core::borrow::{Borrow, BorrowMut};
    use std::thread::JoinHandle;
    use std::fmt::{Error, Formatter, Debug};
    use std::os::raw::c_void;
    use std::sync::mpsc::Receiver;
    use std::cell::{RefCell, Cell};

    const JC_VENDOR_ID: u16 = 1406;
    const LEFT_JC_PROD_ID: u16 = 8198;
    const RIGHT_JC_PROD_ID: u16 = 8199;

    pub type InputReportHandler = Fn(&[u8]) -> () + 'static + Send + Sync;

    trait Threaded {
        fn start(&self);
    }

    struct InputReportHandlerThread{
        input_report_handlers: Vec<Arc<Box<InputReportHandler>>>,
        // Is there a way for me to just concurrently share a HidDevice reference? Is it even safe for me to open a hid multiple times? (input thread and output thread
        input_device: Arc<HidDeviceInfo>,
        join_handle: Option<JoinHandle<()>>
    }

    //Will find a better way to implement traits for the threading structs. Perhaps find a better way to implement the threading!
    impl InputReportHandlerThread{
        fn new(async_handle: Arc<HidDeviceInfo>) -> InputReportHandlerThread{
            InputReportHandlerThread{
                input_report_handlers: vec![],
                input_device: Arc::clone(&async_handle),
                join_handle: None //Needs to be started after initialization
            }
        }

        fn start(&mut self, manager: Arc<JoyconManager>){
            let input_device = self.input_device.clone();
            let input_report_handlers = self.input_report_handlers.clone();
            let thread = thread::spawn(move||{
                let manager = manager.clone();
                let dev = manager.hidapi.open(input_device.vendor_id, input_device.product_id).unwrap();
                loop{
                     let mut buf: &mut [u8] = &mut [0u8; 50];
                     dev.read(&mut buf).unwrap();
                     for handler in &input_report_handlers{
                         handler(buf)
                     }
                }
            });
            self.join_handle = Some(thread);
        }

        fn add_input_handler(&mut self, callback: Box<InputReportHandler>){
            self.input_report_handlers.push(Arc::new(callback))
        }
    }

    pub struct Joycon {
        handle: Arc<HidDeviceInfo>,
        input_thread_join_handle: InputReportHandlerThread,
        output_thread_join_handle: Arc<JoinHandle<Option<bool>>>,
    }

    pub struct JoyconProperties{
        input_report_handlers: Vec<fn(buf: &[u8])>,
    }

    impl Joycon{
        pub fn new(mut manager: Arc<JoyconManager>, handle: HidDeviceInfo) -> Option<Joycon>{
            let manager_clone = Arc::clone(&manager);
            let mut async_handle = Arc::new(handle);
            let device = Arc::new(Arc::clone(&manager_clone).hidapi.open(async_handle.vendor_id, async_handle.product_id).unwrap());
            let mut input_thread = InputReportHandlerThread::new(Arc::clone(&async_handle));
            let output_thread = Arc::new(thread::spawn::<_,Option<bool>>(move || { Some(true) }));
            let mut joycon = Joycon {
                handle: Arc::clone(&async_handle),
                input_thread_join_handle: input_thread,
                output_thread_join_handle: output_thread.clone()
            };
            let joycon_ref = Rc::new(RefCell::new(joycon));
//            let mut jc_clone = Rc::clone(&jc);
//            let mut jc_input_thread = Rc::clone(&jc_clone).input_thread_join_handle;
            Rc::clone(&joycon_ref).into_inner().input_thread_join_handle.start(Arc::clone(&manager_clone));
            return Some(joycon_ref.into_inner());
        }

        pub fn add_input_report_handler_cb(&mut self, callback: Box<InputReportHandler>){
            let (sender, receiver) = std::sync::mpsc::channel::<Box<InputReportHandler>>();
            sender.send(callback);
            self.input_thread_join_handle.add_input_handler(receiver.recv().unwrap());
        }
    }

    pub type JoyconResult = Result<Joycon, Error>;

    pub struct JoyconManager{
        hidapi: HidApi,
        pub connected_joycons: Vec<Joycon>
    }

    impl JoyconManager{
        pub fn new() -> std::result::Result<Arc<JoyconManager>, HidError>{
            let _hidapi = match HidApi::new(){
                Ok(h) => h,
                Err(e) => return Err(e)
            };

            let manager = JoyconManager{
                hidapi: _hidapi,
                connected_joycons: Vec::new()
            };
            let mut jm = Arc::new(manager);
            JoyconManager::search_for_joycons(Arc::clone(&jm));
            Ok(Arc::clone(&jm))
        }

        pub fn get_async(mut this: Rc<JoyconManager>) -> Arc<Mutex<JoyconManager>>{
            let jm = Rc::try_unwrap(this);
            match jm{
                Ok(man) => return Arc::new(Mutex::new(man)),
                Err(_) => panic!("Could not convert Rc<JoyconManager> to Arc<JoyconManager>.") // @TODO: ImproveErr
            }
        }

        /// Searches for joycons and pushes them to `connected_joycons`
        /// Can be used for refreshing the list of joycons, however, this is called internally by a refresh thread
        /// And called upon JoyconManager initialization
        pub fn search_for_joycons(this: Arc<JoyconManager>){
            let mut this_clone = Arc::clone(&this);
            let devs = this_clone.hidapi.devices();
            for _d in devs.iter() {
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
                    let jc = Joycon::new(Arc::clone(&this), d);
                    match jc {//TODO: Improve error handling and referencing!!!!
                        Some(j) => {
                            this.connected_joycons.push(j);
                            true
                        },
                        None => {
                            eprintln!("Error: Could not initialize Joycon."); // @TODO: ERRImprove
                            false
                        }
                    };
                }
            }
        }
    }
}*/
