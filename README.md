# tedium

TDM telephony experimentation

# Installation

This project uses [Poetry](https://python-poetry.org/) to manage Python dependencies.

```bash
# Grant access through `udev` to the FTDI JTAG interface on the Tedium board.
$ sudo ln -sf /home/jboone/src/tedium/99-tedium.rules /etc/udev/rules.d/
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
#$ poetry run tedium/gateware/tedium-fpga
# LUNA_AVOID_BLOCKRAM required for reliable USB enumeration as of 2022/May/02.
$ LUNA_PLATFORM="tedium.gateware.xplatform:TediumX8Platform" LUNA_AVOID_BLOCKRAM=1 tedium/gateware/tedium-fpga
# Initialize the framer/LIU
#$ poetry run tedium/tedium-tool reset
#$ tedium/tedium-tool reset
$ cargo run --release --bin tedium-tool -- init
$ cargo run --release --bin tedium-tool -- monitor
```

NOTE: My Tedium .venv got weird when I had the "OSS CAD Suite" environment already enabled.
Poetry would download dependencies on `poetry install` or `poetry update`, but would then
delete them when running `poetry run` or `poetry shell`.

# PipeWire

Running a stand-alone PipeWire instance:

```bash
# Absolute path is assential, or it'll load relative to some `share` directory.
$ PIPEWIRE_DEBUG="D" pipewire -c /home/jboone/src/tedium/pipewire.conf
$ PIPEWIRE_REMOTE="pipewire-tedium" pw-top
$ PIPEWIRE_REMOTE="pipewire-tedium" pw-cli
$ PIPEWIRE_REMOTE="pipewire-tedium" pw-dot -a -d && dot -Tsvg pw.dot -o pw.svg && open pw.svg
# Note that qpwgraph doesn't seem to have a way to specify the non-default PipeWire instance (remote).
```

# USB Debugging

```bash
$ sudo cat /sys/kernel/debug/usb/devices
$ sudo cat /sys/kernel/debug/dynamic_debug/control | grep "snd_usb_audio"
$ sudo echo 'file sound/usb/* +p' >/sys/kernel/debug/dynamic_debug/control
$ sudo sysctl -w kernel.printk=7 # debug-level messages
$ sudo sysctl kernel.printk
```

## Monitoring ALSA Device State

```bash
$ watch -n 1 cat /proc/asound/cardX/stream0
...
ShareBrained Tedium X8 at usb-0000:00:14.0-2, high speed : USB Audio

Playback:
  Status: Running
    Interface = 1
    Altset = 1
    Packet Size = 384
    Momentary freq = 8000 Hz (0x1.0000)
  Interface 1
    Altset 1
    Format: MU_LAW
    Channels: 192
    Endpoint: 0x01 (1 OUT) (ASYNC)
    Rates: 8000
    Data packet interval: 125 us
    Bits: 8
    Sync Endpoint: 0x81 (1 IN)
    Sync EP Interface: 2
    Sync EP Altset: 1
    Implicit Feedback Mode: Yes

Capture:
  Status: Running
    Interface = 2
    Altset = 1
    Packet Size = 384
    Momentary freq = 8000 Hz (0x1.0000)
  Interface 2
    Altset 1
    Format: MU_LAW
    Channels: 192
    Endpoint: 0x81 (1 IN) (ASYNC)
    Rates: 8000
    Data packet interval: 125 us
    Bits: 8
```

Critical implicit feedback details being the `Sync EP` values, which look correct in the example above.

What's odd though is the `Packet Size` numbers, which I guess are the 1 + additional packets per frame * the max packet size? In which case they make sense. But that's not what the descriptor says.

## Using Wireshark

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

# Trouble With Real-Time Kit

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

# Use

See [the software README](tedium/README.md) for information on how to use `tedium-tool` to interact with the hardware over USB.

### License

This software and gateware in this project is licensed under the [BSD 3-Clause License](LICENSE-BSD-3-Clause).

The hardware is licensed under the [CERN Open Hardware Licence Version 2 - Permissive](LICENSE-CERN-OHL-P-v2).
