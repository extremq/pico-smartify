# Pico Smartify
![](https://github.com/user-attachments/assets/27eaf9ea-ea24-452f-92bf-b16fc70c0945)

This project aims to *smartify* any device you want! Plug in a device and control it wherever you go.

![](https://github.com/user-attachments/assets/ae4c589b-c79b-40b9-801c-229bbd6f081d)

## What we used
- Raspberry Pi Pico 2W + Raspberry Pi Pico W as debug probe;
- 5V relay capable of 10A control;
- Extension cord;
- Breadboards & wires;
- Headers for the microcontrollers;
- [Embassy](https://github.com/embassy-rs/embassy).

## Wiring
First, we wired the Pico W as the debug probe for the Pico 2W, as per the reference:

![image](https://github.com/user-attachments/assets/dd22edca-0eeb-419e-805f-4ff08c14b9ff)

Connect the relay with the VCC at the 5V output, GND at any Pico GND you desire and the IN can be connected to any GPIO you want. We chose GPIO6.

After that, you must cut the extension cord wire and connect it appropiately in the NO and COM ports of the relay.

> [!CAUTION]
> Please take extra caution when handling the wires of the extension cord. Make sure it is NOT plugged in while you handle it. Make sure you screw the wire well into the relay.
> Do not attempt this without a bit of researching before. **NEVER TOUCH THE CONNECTION WHILE THE PROJECT IS RUNNING**.

![image](https://github.com/user-attachments/assets/9060a418-a6b8-4c5c-8eae-ef1138147742)

## Building & Running
We will make use of Rust. To install it, we will use Rustup:
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

To check it's installed, please run the following command:
```
rustup --version
```
The output should be similar:
```
rustup 1.28.2 (e4f3ad6f8 2025-04-28)
info: This is the version for the rustup toolchain manager, not the rustc compiler.
info: The currently active `rustc` version is `rustc 1.87.0 (17067e9ac 2025-05-09)`
```

We will make use of a program that loads the code for us instead of having to phisically disconnect the Pico each time. We will use [elf2uf2-rs](https://github.com/JoNil/elf2uf2-rs):
```
cargo install elf2uf2 --locked
```
To check, run:
```
elf2uf2-rs --help
```
You should have something similar:
```
Usage: elf2uf2-rs [OPTIONS] <INPUT> [OUTPUT]

Arguments:
  <INPUT>   Input file
  [OUTPUT]  Output file

Options:
  -v, --verbose  Verbose
  -d, --deploy   Deploy to any connected pico
  -s, --serial   Connect to serial after deploy
  -t, --term     Send termination message to the device on ctrl+c
  -h, --help     Print help
```

Alright. To program and debug the Pico, we will need [probe-rs](https://probe.rs/):
```
curl -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
```
Run:
```
probe-rs --version
```
You should have:
```
probe-rs 0.29.0 (git commit: cc6b448)
```


Also, add this [file](https://probe.rs/files/69-probe-rs.rules) in `/etc/udev/rules.d/` and run the following commands with admin privilege:
```
udevadm control --reload
udevadm trigger
```

You can check for the Pico, if connected, by running:
```
probe-rs list
```
Example output:
```
The following debug probes were found:
[0]: Debugprobe on Pico (CMSIS-DAP) -- 2e8a:000c:E6614103E7457C37 (CMSIS-DAP)
```

You also need to add this toolchain to your rustup installation, as we are compiling for a specific architecture:
```
rustup target add thumbv8m.main-none-eabihf
```

As per [Updating the firmware on the Debug Probe](https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html#updating-the-firmware-on-the-debug-probe),
you will need to download the firmware for the Debug Probe that the Pico W will serve as. Please follow their instructions.

Alright, now that everything is ready, all that is left to do is run the project:
```
cargo run
```
The program should be flashed:
```
    Finished `release` profile [optimized + debuginfo] target(s) in 6.93s
     Running `probe-rs run --chip RP235x target/thumbv8m.main-none-eabihf/release/pico-smartify`
      Erasing ✔ 100% [####################] 380.00 KiB @ 101.34 KiB/s (took 4s)
  Programming ✔ 100% [####################] 380.00 KiB @  47.17 KiB/s (took 8s)
     Finished in 11.81s
```
After some time, you will have this:
```
15.019221 [DEBUG] IPv4: UP (embassy_net embassy-net-0.7.0/src/lib.rs:732)
15.019240 [DEBUG]    IP address:      192.168.3.105/24 (embassy_net embassy-net-0.7.0/src/lib.rs:733)
15.019284 [DEBUG]    Default gateway: Some(192.168.3.1) (embassy_net embassy-net-0.7.0/src/lib.rs:734)
15.019320 [DEBUG]    DNS server:      192.168.3.1 (embassy_net embassy-net-0.7.0/src/lib.rs:740)
```
Connect to the IP address in a browser, or use curl:
```
curl -X POST http://192.168.3.105/on
Already powered on!⏎ 
curl -X POST http://192.168.3.105/off
Powered off!⏎ 
curl -X POST http://192.168.3.105/
POST /on, /off, /status.⏎
curl -X POST http://192.168.3.105/status
0⏎
curl -X POST http://192.168.3.105/on
Powered on!⏎
curl -X POST http://192.168.3.105/status
1⏎
```

That's it! You can also use multiple devices!

