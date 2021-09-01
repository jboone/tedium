
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
