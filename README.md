# joycon-rs
A native implementation of a clean, efficient, high-quality, asynchronous API for Nintendo Switch Joycon support in desktop games.

The following README is currently planned implementation.

This API will have a JoyconManager for easily and safely managing the connected Joycons. It will handle the hardware side of things such as handling when a Joycon is detected, paired/connected, disconnected, low on battery, or in shared mode.
The JoyconManager will continuously and silently scan for Joycons with zero-overhead in a background thread. When a Joycon is connected, it will initialize a new Joycon object which will do the following:
  * Create a main Joycon thread which will contain child threads for delegated parallel functionality of handling input and output reports.
  * Read the SPI flash memory and fetch the default factory calibration for analog and gyroscope/accelerometer (if user calibration is not set).
  * Fetch hardware information such as current battery information (more hardware info later on if needed).
  * Create mappings of the buttons based on whether it is in shared mode or not (see section on shared mode).

A Joycon object will contain methods for requesting an action (or sending an output report), rumbling the joycon (a type of action request), getting the current battery information, reading memory (a type of action request), setting player led (player 1, player 2, etc), etc. Every Joycon object will also allow enabling shared mode and have a "companion joycon". Shared mode will be handled by the JoyconManager.

# Shared Mode
Shared mode is a not really a mode in the firmware but really an implementation mode. Shared mode is a simple feature of the Joycon that allows the two devices to act independently of each other, becoming two separate controllers. This is *very* important because then the input mappings, by the interpreting software/game, have to be rotated by 90-degrees (negative for left controller, positive for the right controller). To handle this properly, the [API architecture](ARCHITECTURE.md) will need to carefully implement a user-friendly handle.
