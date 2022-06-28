# tedium-soc

This is the firmware that runs on the "system-on-chip" (SoC) on the FPGA, and works closely with the framer IC to provide a tidy and expeditious interface to the host over USB.

## Building

As of Rust 1.61, the `.cargo` configuration for this firmware sub-project will be ignored if you compile from the project root, where the Cargo.toml that itemizes the project workspaces. You will need to build from the directory this `README.md` is in, in order to have cargo compile for the correct RISC-V architecture.

## Downloading to FPGA

This firmware's binary is loaded via the `lambdasoc` BIOS running on the RISC-V in the FPGA.

```bash
$ cargo build --release
$ riscv32-unknown-linux-gnu-objcopy -Obinary ../../target/riscv32i-unknown-none-elf/release/tedium-soc tedium-soc.bin
$ python ~/src/tedium-project/lambdasoc/lambdasoc/tools/flterm.py --kernel tedium-soc.bin /dev/ttyUSB2
BIOS> serialboot
```
