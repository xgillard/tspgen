# tspgen

A generator of clustered TSP instances.

## Usage 

```
Generates clustered TSP instances

Usage: tspgen [OPTIONS]

Options:
  -n, --nb-cities <NB_CITIES>        The number of cities that must be visited [default: 10]
  -c, --nb-centroids <NB_CENTROIDS>  The number of centroids that must be visited [default: 3]
  -m, --max <MAX>                    The maximum width of the generated map [default: 1000]
  -d, --std-dev <STD_DEV>            The std deviation between a city and its centroid [default: 10]
  -s, --seed <SEED>                  An optional seed to kickstart the instance generation
  -h, --help                         Print help information
  -V, --version                      Print version information
```

## Build

`cargo build --release`