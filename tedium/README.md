
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

# USB Monitoring for Debug

Assuming Ubuntu Linux, employing [ktemkin's guide](https://usb.ktemkin.com/usbmon):

```bash
$ sudo modprobe usbmon
$ lspci -tv
# Identify "Bus" <n> that target device (vid:pid:name) is on.
```

Launch Wireshark and choose the usbmon<n> to capture from. You may need to `sudo dpkg-reconfigure wireshark-common` and permit non-superusers to be able to capture packets, and add yourself to the `wireshark` group, then log out/in. Wireshark GUI will provide a reminder dialog about this if you try to open a usbmon interface without adequate permissions.

You might also need to add udev permissions for `plugdev` to the `usbmon` devices:

```bash
echo 'SUBSYSTEM=="usbmon", GROUP="plugdev", MODE="640"' | sudo tee /etc/udev/rules.d/50-accessible-usbmon.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```
