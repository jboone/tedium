# tedium

TDM telephony experimentation

## Dependencies

* `oss-cad-suite`: open-source digital design and verification tools, with [nightly binary builds](https://github.com/YosysHQ/oss-cad-suite-build).
* [`poetry`](https://python-poetry.org/): Python dependencies and packaging management tool.

## Installation

Clone this repository, and generally follow these steps:

```bash
$ cd tedium
# Grant access through `udev` to the FTDI JTAG interface on the Tedium board.
$ sudo ln -sf ./99-tedium.rules /etc/udev/rules.d/
$ sudo udevadm control --reload-rules && sudo udevadm trigger
#
$ pip3 install poetry --user
# Might need to log out and in to get ~/.local/bin in PATH.
# Or, `source ~/.profile`.
# Set up virtual environment, download and install dependencies
$ poetry install
# Enter poetry shell
$ poetry shell
# Include OSS CAD Suite tools in PATH
$ export PATH="/home/jboone/src/fpga/oss-cad-suite/bin:$PATH"
# Build and download FPGA bitstream
# LUNA_AVOID_BLOCKRAM required for reliable USB enumeration as of 2022/May/02.
$ LUNA_AVOID_BLOCKRAM=1 applets/tedium-fpga
# After the bitstream is generated, you can inspect the results in the `build` directory.
# Initialize the framer/LIU
$ cargo run --release --bin tedium-tool -- init
# Run the host-side audio and event engine
$ cargo run --release --bin tedium-tool -- monitor
```

NOTE: My Tedium .venv got weird when I had the "OSS CAD Suite" environment already enabled.
Poetry would download dependencies on `poetry install` or `poetry update`, but would then
delete them when running `poetry run` or `poetry shell`.

## Testing

Unit tests may be run like this:

```bash
$ python3 -m unittest tests.hdl.tx_usb_to_fifo
# Or the name of any other HDL test module you want to run/simulate.
```

## USB Debugging

```bash
$ sudo cat /sys/kernel/debug/usb/devices
$ sudo cat /sys/kernel/debug/dynamic_debug/control | grep "snd_usb_audio"
$ sudo echo 'file sound/usb/* +p' >/sys/kernel/debug/dynamic_debug/control
$ sudo sysctl -w kernel.printk=7 # debug-level messages
$ sudo sysctl kernel.printk
```

### Using Wireshark

```bash
$ sudo modprobe usbmon
$ sudo wireshark
# Capture stuff
```

Wireshark filters that might be of use:

```
# Find USB IN on endpoint 1, where the frame of completed inbound URBs is not the typical length.
# This helps find where the async audio input FIFO level is wrapping around.
usb.endpoint_address.number==1 && usb.endpoint_address.direction==IN && usb.urb_type=='C' && frame.len != 1728
# Set the time reference at an interesting record in Wireshark, disable view filtering, and work from there.
# Note that "marked" records appear regardless of whether they match the view filter, which is very handy.
# Now match on USB OUT records that have been submitted (pending transmission)
usb.endpoint_address.number==1 && usb.endpoint_address.direction==OUT && usb.urb_type=='S'
```

However, it seems like Wireshark isn't showing complete URB buffer contents?

## Trouble With Real-Time Kit

```bash
# Is there an `rtkit` user?
$ getent passwd rtkit
# Is there an `rtkit*` user?
$ ps ax | grep -i rtkit
# Check that RTKit is actually changing our process/thread priority:
$ journalctl -e -t rtkit-daemon
# `ulimit` values:
$ ulimit -a
```

## Use

See [the software README](tedium/README.md) for information on how to use `tedium-tool` to interact with the hardware over USB.

## License

This software and gateware in this project is licensed under the [BSD 3-Clause License](LICENSE-BSD-3-Clause).

The hardware is licensed under the [CERN Open Hardware Licence Version 2 - Permissive](LICENSE-CERN-OHL-P-v2).
