# Bus RAPTOR

A backward [RAPTOR](https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf) (Round-based Public Transit Routing) implementation in Rust for private bus services. Given a destination and target arrival time, it finds the latest possible departure across all stops.

## Build

```bash
cargo build --release
```

## Usage

```bash
cargo run -- \
  --stops data/stops.csv \
  --routes data/routes.csv \
  --destination "Kaminoge Station" \
  --arrive-by "09:00"
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--stops`, `-s` | Stops CSV file | required |
| `--routes`, `-r` | Routes CSV file | required |
| `--destination`, `-d` | Destination stop (name or ID) | required |
| `--arrive-by`, `-a` | Target arrival time (`HH:MM`) | required |
| `--transfers`, `-t` | Transfers CSV file | auto-generated |
| `--max-transfers` | Maximum allowed transfers | 3 |
| `--walking-distance` | Max walking distance in meters | 300 |
| `--walking-speed` | Walking speed in km/h | 5.0 |
| `--bus-speed` | Bus speed in km/h | 30.0 |
| `--max-departure-delay` | Max departure delay in minutes | 60 |
| `--output-geojson` | Write route to GeoJSON file | - |
| `--verbose`, `-v` | Verbose output | false |

### Private bus mode

Plan a journey through specific stops:

```bash
cargo run -- private \
  --stops data/stops.csv \
  --routes data/routes.csv \
  --stop-list "Denen-Chofu Station" "Oyamadai" "Seta" \
  --destination "Destination" \
  --arrive-by "09:00"
```

## Data format

**stops.csv**
```csv
id,name,lat,lon
1,Denen-Chofu Station,35.6001,139.6668
```

**routes.csv**
```csv
id,name,stops,travel_times
1,Bus 1,"1,2,3,4,5,6,7,8,9","180,480,780,360,420,300,2400,1800"
```

`stops` lists stop IDs in order. `travel_times` gives seconds between consecutive stops.

Walking transfers between nearby stops are generated automatically from the Haversine distance.

## Tests

```bash
cargo test
```

## How it works

The backward RAPTOR algorithm starts at the destination with the target arrival time and works backwards round by round:

1. Set the destination's arrival time
2. For each round (one more transfer allowed), scan all routes to find the latest departure at each reachable stop
3. Propagate walking transfers between nearby stops
4. Reconstruct the journey from parent pointers

Uses `rayon` for parallel route processing and `bitvec` for efficient stop marking.
