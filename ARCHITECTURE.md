The Joycon-API specification is not complete and is heavily subject to change.<br>

This API specification contains all the information you need to know about all the components of the architecture of this API.<br>

### Joycon Representation Object (JRO)
* A JRO is simply just representing a Joycon and must remain minimal in size and carry low overhead
* A JRO must be or must have a main thread that spawns child threads
    * An input thread that handles all the input report handler callbacks and creates the necessary parsed input reports. See [input report wrappers](#input-report-wrappers-irw) for more information.
    * An output thread that handles all the action requests (see action specification), validates the action requests (see action validation), and sends the output reports
* A JRO must have methods for creating rumble patterns (in the form of lambdas/callbacks), reading memory (either as lambdas/callbacks or through a memory reader (see memory reader specification)), and getting battery information
* A JRO must also have a HID manager for handling disconnecting/unpairing and HID errors
* A JRO must also have a proper error handling or crash reporting system
    * The error handling or crash reporting system will be either default error handling or crash reporting or registered custom handlers through the JoyconManager

### JoyconManager
* A JoyconManager must spawn a joycon scanner thread that silently and efficiently (low overhead) scans for newly paired/connected joycons
    * This thread must also monitor the currently paired/connected joycons for unpairing/disconnecting
* A JoyconManager must have a way of enabling shared mode by not exposing the process to dependents
    * Shared mode should be implemented by having the "connected_joycons" field (currently a `Vec<Joycon>`) be of type `HashMap<Joycon, Joycon>` and "null joycons" should be `DummyJoycon`
* A JoyconManager must provide a calibration factory or provide some way of building custom calibrations (does not need to be a "factory")
* A JoyconManager must have a crash report or error handling registration
* A JoyconManager must provide a rumble factory
    * The rumble factory can be implemented in any way.
    * Plans for using JSON for constructing packets before parsed into an action is considered for the default implementation
    * See [rumbling](#rumbling) below

### Actions
* An action is an output report that is wrapped in an object that the API can understand
* An action report should be readable by the user and the Joycons by being represented in two ways
    * The buffered output report in the form of a u8 array sent to the Joycon devices
    * The preconstructed object containing information on how to build the buffered output report
    * Actions include, but is not limited to:
        * Rumbling (see rumble patterns and rumble factory specs)
            * This is not written up yet. Rumbling is very technical as it requires the data to contain a high frequency, low frequency, amplitude, and duration which is calculated in a certain way. The maths for this can be viewed [here](https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/rumble_data_table.md).
        * Setting player number LED
        * Changing the home button light
        * Request to read memory (see memory manager)
        * Request to write memory (such as writing user custom calibrations; see memory manager)
        * Retrieve the current battery voltage and parse into a `BatteryInformation` property
    * A more detailed document on output reports and how they are constructed internally will come soon.

### Input Report Handlers
 * Input Report Handler Callbacks
 * Callbacks that are passed the input report must be handled on the joycon's dedicated input thread
 * Callbacks must be registered during initialization of the joycons via the JoyconManager.

### Input Report Wrappers (IRW)
 * Input Reports must be wrapped in an object that parses the data into something that can be used
 * Input Reports are raw data and must be handled internally and sorted into their respective input report wrappers
 * Input Report Wrappers must implement a report parsing algorithm that takes the input report raw data and converts it into something that can be used by dependents
 * A more detailed document on how raw input reports are parsed into IRW's will come soon. Please refer to [these notes](https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#input-reports) for more information on how input reports work in USB/Bluetooth Specification. 

### Action requests
* An action request is an output report that is wrapped in an object and passed into a queue to be sent to the joycon
* An action request is validated by the specified joycon's dedicated output thread
* See [actions](#actions) for information on actions
  
  * ##### Rumbling
    - A rumble is a vibration frequency (low and high) and duration (see simple rumble packet below) sent to the Joycon device
    - A rumble must be sent in the form of a rumble packet, which can be either a simple packet or a complex packet
        - A simple rumble packet contains a high frequency, low frequency, duration, and device target
        - A complex rumble packet should be a collection of simple rumble packets that form a rumble pattern
    - A rumble packet can either be for a single device or several devices
    - A more detailed explanation of rumbling is coming soon.
  * Memory manager
    - A memory manager must be present within a JRO for safe, secure, and efficient memory reading/writing
    - A more detailed document on how memory management works will come soon. Please refer to [these notes](https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/spi_flash_notes.md) and [these notes](https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_subcommands_notes.md#subcommand-0x10-spi-flash-read) for information on how memory works in Nintedo Switch Joycons.
