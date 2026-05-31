# MouseKey
(Linux only at this point!)

Takes your keyboard and mouse and merges them into a virtual device, mousekey, with all of the same capabilities except it is now one unique device.
Also converts mouse movements into arrow keys. Moving the mouse in a direction will send 1 arrow key movement, and then moving the mouse further will start repeating that key, which will stop if you move the mouse in the opposite direction. Distance you must move to activate/ repeat in `config.toml` (in same folder as the `mousekey` executable)

## IMPORTANT NOTE!! (DO NOT PROCEED UNTIL YOU HAVE READ THIS!)
Force quit keybind: F5+F7+F8
Mouse passthrough: Hold Caps Lock

**WARNING**: This program redirects all of the inputs from the devices you select to its 'mousekey' uinput device.
Please be prepared to force quit the terminal and possibly power down your device before proceeding.

## Installation

todo

## Running the program
from within the installation folder, open a terminal and run:
```
./mousekey
```
And follow the instructions it gives you. It does not require `sudo` for me, but I am honestly not sure why, I would have thought that accessing /dev/input would always require sudo.

If it does not work, try
```
sudo ./mousekey
```
And if it still does not work, submit an issue in https://github.com/jammie1thelegend/mousekey/issues, and I will see if I can fix it.

## Configuration
You can tweak the move distance (`move_limit`) and repeat distance (`hold_time`) in `config.toml`, which should be in the same folder as the `mousekey` executable.

Default `config.toml`:
```
move_limit = 40
hold_time = 45
```

## Building from Source
Simple rust program, make sure to have rust and cargo installed, then run:
```
git clone https://github.com/jammie1thelegend/mousekey
cd mousekey
cargo build
```
which will clone the repo, navigate to it, and run `cargo build`.