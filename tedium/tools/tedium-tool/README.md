# tedium-tool

This thing needs its own readme.

`tedium-tool` interacts with the Tedium hardware over USB. It mainly pumps audio and events back and forth over USB, and tries (and presently fails miserably) at pretending to be a telephone switch.

## Running

```bash
$ cargo run --bin tedium-tool --release -- monitor
```
