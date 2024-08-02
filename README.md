# loadrs — server user load monitoring tool in Rust
[![Crates.io](https://img.shields.io/crates/v/loadrs.svg)](https://crates.io/crates/loadrs)

![loadrs in action](https://raw.githubusercontent.com/vsbuffalo/loadrs/main/screenshot.png)

This is a simple tool that monitors and displays CPU usage by user on a Linux
server. It's implemented in Rust for efficiency and reliability. This was
co-written with Claude AI.

Key features:

1. Real-time monitoring: Continuously updates CPU usage information with `-l/--live`.
2. User-specific data: Breaks down CPU usage by individual users.
3. Fair share calculation: Determines what constitutes fair CPU usage based on active users or a specified proportion.
4. Color-coded output: Easily identify users exceeding their fair share of CPU resources.

## Usage

Run the tool with default settings:

```bash
load
```

To run in a *live* mode (updated every 5 seconds), use `load -l`.

The fair share calculation is based on the number of *active users* where
active is defined as using over `--active-threshdold` percent CPU usage (by
default, 1%). If the fair share should be a fixed percentage, use `--fair-share
20` for 20%. If there is an excess load (based on the one minute load average
and the `--active-threshdold`), a list of users will be displayed using more
than their fair share. This is not meant to blame users, but help them be aware
of their resource usage. Here we also set the warning threshold to 80% of total
CPUs:

```bash
load --fair-share 20 --threshold 80
```
