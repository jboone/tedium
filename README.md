# tedium

TDM telephony experimentation

# Installation

This project uses [Poetry](https://python-poetry.org/) to manage Python dependencies.

```bash
$ pip3 install poetry --user
# Might need to log out and in to get ~/.local/bin in PATH.
# Or, `source ~/.profile`.
# Set up virtual environment, download and install dependencies
$ poetry install
# Build and download FPGA bitstream
$ poetry run tedium/gateware/tedium-fpga
```

# Use

See [the software README](tedium/README.md) for information on how to use `tedium-tool` to interact with the hardware over USB.

### License

This software and gateware in this project is licensed under the [BSD 3-Clause License](LICENSE-BSD-3-Clause).

The hardware is licensed under the [CERN Open Hardware Licence Version 2 - Permissive](LICENSE-CERN-OHL-P-v2).
