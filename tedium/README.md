
# Hardware Setup

The anticipated use case will be for Tedium to recover the clock on span 0 and distribute it to all spans (including span 0 TX).

* Tedium span 0: T-BERD 950 "Line" (primary system clock source)
* Tedium span 1: Adit 1 span 0 (primary source for Adit 1)
* Tedium span 2: Adit 1 span 1 (secondary source for Adit 1)

The T-BERD should be set up accordingly:

* Main
  * Interface: T1
  * Test Type: VOICE

* Setup
  * Mode: TERMINATE
  * Framing: ESF
  * Line Coding: B8ZS
  * Line Rx In: TERM
  * Tx/Rx Pair: LINE
  * Tx Timing: INTERNAL

Don't use the T-BERD "Equipment" DDS-LL port, as it's pretty certain it doesn't do what I'd expect and need it to do.

The Adit 600 should be set up accordingly:

"""
set a:1 up
set a:1 framing esf
set a:1 linecode b8zs
set a:1 lbo 1
set a:1 fdl t1403
set a:1 loopdetect csu
set a:1 payload loopdown
set a:1 ais disable

set a:1:all type voice
set a:1:all signal ls

# Transmit clocks, primary and secondary
# Use the first span RX as the TX source, then the second span RX.
set clock1 A:1
set clock2 A:2

# ???
show idle
show autoexit
set autoexit off
show local
set local off
log
show screen
set screen 75
set screen off
"""

Review the configuration:

"""
show a:1
show a:2

show connect a:all

show clock
status clock
"""

# Configuration

To run `tedium-tool`, you need `python3-usb` (Debian/Ubuntu).

You'll also likely want to add a `udev` rule to allow non-superuser access to the Tedium USB interface.

```bash
$ sudo vi /etc/udev/rules.d/99-luna.rules
```

Edit the file to contain:

```udev
ATTRS{idVendor}=="16d0", ATTRS{idProduct}=="0f3b", MODE="0666", GROUP="plugdev", TAG+="uaccess"
```

Then reload the rules:

```bash
$ sudo udevadm control --reload-rules
$ sudo udevadm trigger
```

# Building and Downloading the Gateware

__TODO__: These are notes from my temporary setup. Fill this out when everything stabilizes a bit.

```bash
PYTHONPATH=.:../amaranth:../amaranth-soc:./amaranth-stdio:../lambdasoc:../luna:../minerva python applets/tedium-fpga
```

To reload the gateware on the FPGA (because something crashed or it otherwise can't be negotiated with), this saves having to rebuild the whole bitstream:

```bash
$ openocd -f build/design/top-openocd.cfg -c 'init; svf -quiet build/design/top.svf; exit'
```

# Framer/LIU Testing

Upon loading the gateware into the FPGA, you'll need to reset the framer/LIU chip.

```bash
$ ./tedium-tool reset
```

Dump the current state of the framer/LIU chip's registers:

```bash
$ ./tedium-tool dump
```

Transmit unframed BERT w/Daly pattern:

```bash
$ ./tedium-tool write 0x0163 0b0000_1001
$ ./tedium-tool write 0x0121 0b0000_0001
$ ./tedium-tool write 0x0123 0b0000_0100
```

Also receive and check BERT pattern:

```bash
$ ./tedium-tool write 0x0163 0b0000_1001
$ ./tedium-tool write 0x0121 0b0000_0001
$ ./tedium-tool write 0x0123 0b0000_1100
```

I think I finally understand the D/E time slots mentioned in the T1 registers documentation. I think that allows you to configure any combination of RX/TX time slots as an LAPD "stream".

# USB Monitoring for Debug

Assuming Ubuntu Linux, employing [ktemkin's guide](https://usb.ktemkin.com/usbmon), or the [Linux kernel documentation for usbmon](https://www.kernel.org/doc/Documentation/usb/usbmon.txt):

```bash
$ sudo modprobe usbmon
$ lsusb -tv | grep "16d0:0f3b"
# Identify "Bus" <n> that target device (vid:pid:name) is on.
...
Bus 003 Device 045: ID 16d0:0f3b MCS Tedium X8
...
$ sudo cat /sys/kernel/debug/usb/devices
...
T:  Bus=03 Lev=01 Prnt=01 Port=02 Cnt=03 Dev#= 45 Spd=480  MxCh= 0
D:  Ver= 2.00 Cls=00(>ifc ) Sub=00 Prot=00 MxPS=64 #Cfgs=  1
P:  Vendor=16d0 ProdID=0f3b Rev= 1.01
S:  Manufacturer=ShareBrained
S:  Product=Tedium X8
C:* #Ifs= 3 Cfg#= 1 Atr=c0 MxPwr=100mA
I:* If#= 0 Alt= 0 #EPs= 1 Cls=ff(vend.) Sub=ff Prot=ff Driver=(none)
E:  Ad=89(I) Atr=03(Int.) MxPS=  64 Ivl=1ms
I:* If#= 1 Alt= 0 #EPs= 0 Cls=ff(vend.) Sub=ff Prot=ff Driver=(none)
I:  If#= 1 Alt= 1 #EPs= 1 Cls=ff(vend.) Sub=ff Prot=ff Driver=(none)
E:  Ad=01(O) Atr=05(Isoc) MxPS= 384 Ivl=125us
I:* If#= 2 Alt= 0 #EPs= 0 Cls=ff(vend.) Sub=ff Prot=ff Driver=(none)
I:  If#= 2 Alt= 1 #EPs= 1 Cls=ff(vend.) Sub=ff Prot=ff Driver=(none)
E:  Ad=82(I) Atr=05(Isoc) MxPS= 384 Ivl=125us
...
```

Launch Wireshark and choose the usbmon<n> to capture from. You may need to `sudo dpkg-reconfigure wireshark-common` and permit non-superusers to be able to capture packets, and add yourself to the `wireshark` group, then log out/in. Wireshark GUI will provide a reminder dialog about this if you try to open a usbmon interface without adequate permissions.

You might also need to add udev permissions for `plugdev` to the `usbmon` devices:

```bash
echo 'SUBSYSTEM=="usbmon", GROUP="plugdev", MODE="640"' | sudo tee /etc/udev/rules.d/50-accessible-usbmon.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

# Running Tests

Tests can be run in root of tedium project or a subdirectory. Poetry will provide the correct module path info.

Specify `GENERATE_VCDS=1` if you want GTKwave project and waveform outputs from each test.

```bash
# Make a directory to put test output -- GTKwave project and waveform files.
mkdir tmp
cd tmp
GENERATE_VCDS=1 poetry run python -m unittest tedium.gateware.test.test_enumerate
# Or you can run LUNA tests.
GENERATE_VCDS=1 poetry run python -m unittest luna.gateware.usb.usb2.descriptor
```

# Running New Code on the Tedium SoC

Be careful using the `lambdasoc` BIOS. The memory read/write/copy instructions default to parsing addresses as decimal unless you prefix with `0x`. And if you attempt to access an invalid address, the SoC (or at least the BIOS) hangs.
