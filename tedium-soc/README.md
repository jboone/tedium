# tedium-soc

This is the firmware that runs on the "system-on-chip" (SoC) on the FPGA, and works closely with the framer IC to provide a tidy and expeditious interface to the host over USB.

## Building

As of Rust 1.61, the `.cargo` configuration for this firmware sub-project will be ignored if you compile from the project root, where the Cargo.toml that itemizes the project workspaces. You will need to build from the directory this `README.md` is in, in order to have cargo compile for the correct RISC-V architecture.

```bash
# Make sure the necessary RISC-V Rust compiler is installed.
$ rustup target add riscv32i-unknown-none-elf
# Build the firmware.
$ cargo build --release
# Strip the resulting ELF binary to just the sections to load into the SoC.
$ riscv32-unknown-linux-gnu-objcopy -Obinary target/riscv32i-unknown-none-elf/release/tedium-soc tedium-soc.bin
```

## Downloading to FPGA

This firmware's binary is loaded via the `lambdasoc` BIOS running on the RISC-V in the FPGA.

```bash
$ python ~/src/tedium-project/lambdasoc/lambdasoc/tools/flterm.py --kernel tedium-soc.bin /dev/ttyUSB2
BIOS> serialboot
# ...and the firmware is downloaded and executed by the SoC.
# You may now interact with Tedium via the Rust runtime tools.
```
