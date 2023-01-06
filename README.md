# tsptools

A generator of clustered TSP instances.

## Usage 

```
Usage: tsptools <COMMAND>

Commands:
  generate   TspGen is a generator for realistic TSP instances where the cities to visit are gouped in clusters
  visualize  This command lets you generate an html file to visualize a given instance and an optional solution
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

## Build

`cargo build --release`