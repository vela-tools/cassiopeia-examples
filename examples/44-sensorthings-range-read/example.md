# Reading a time range from a SensorThings server

[Example 43](../43-sensorthings-latest-read/example.md) read the newest reading of each river gauge, one record per gauge. This example reads every reading of the last six hours from the same server. A range read gives one record per Observation, so each gauge has many records. The run folds them into one `WaterObserved` entity per gauge, with one water level instance and one discharge instance per reading. That needs series output, and the run refuses to start without it.

## Get the data

The source is the Thuringian gauge network of [example 43](../43-sensorthings-latest-read/example.md): the FROST-Server at `https://kshww2.thueringen.de/FROST-Server/v1.1`, run by the Thuringian State Office for Environment, Mining and Nature Conservation (TLUBN). TLUBN names itself as the owner of the data and states no licence for the service. The selection is also the same: the 15-minute water level `W` of 127 gauges and the 15-minute discharge `Q` of 123.

```bash
cassiopeia sensorthings readings https://kshww2.thueringen.de/FROST-Server/v1.1 \
    --datastreams "properties/type eq 'TH_PEGELDATEN_15MIN'"
```

```text
reading  unit  unit code  data streams  multi data streams  things with repeats
Q        m³/s  MQS        123           0                   0
W        cm    CMT        127           0                   0
```

A range read asks for the Observations of each Datastream separately. This is the request for the Rothenstein gauge's water level over six hours:

```bash
curl --globoff "https://kshww2.thueringen.de/FROST-Server/v1.1/Datastreams(7668)/Observations?\$filter=phenomenonTime%20ge%202026-11-18T05:00:00Z%20and%20phenomenonTime%20lt%202026-11-18T11:00:00Z&\$orderby=phenomenonTime%20asc,id%20asc&\$select=id,phenomenonTime,result,resultTime,validTime,resultQuality,parameters"
```

Change the two instants to a recent window before you run it.

## Get the schema

```bash
cassiopeia sdm download
```

## How a range read reads

```json5
sensorthings: {
    datastreams: "properties/type eq 'TH_PEGELDATEN_15MIN'",
    read: { range: { last: "6h" } },
},
```

`last: "6h"` reads the six hours before the run starts. The window includes its first instant and excludes the start of the run. `from` and `to`, each an RFC 3339 instant, read a fixed window instead, as [example 49](../49-sensorthings-multidatastreams/example.md) does.

The read sends two kinds of request. The first lists the selected Datastreams, each with its Thing, the Thing's Locations, its ObservedProperty and its Sensor, but without Observations. The `readings` command sends the same request. Then, for each Datastream in ascending id order, the read sends the request above and follows its pages. `$filter` bounds `phenomenonTime` to the range, and `$orderby` puts the oldest first. `$select` asks only for the Observation members a record carries, which keeps the links to each Observation's Datastream and FeatureOfInterest out of the page. A Datastream's own `phenomenonTime` is the interval all of its Observations span, and the read skips a Datastream whose interval ends before the range.

250 Datastreams take 250 requests. The server answers each in a fraction of a second, so the read takes about 20 seconds.

## The record shape

A range read gives one record per Observation, with the same `server`, `thing`, `location` and `locations` as a latest record, and one reading:

```json
{
    "server": {
        "url": "https://kshww2.thueringen.de/FROST-Server/v1.1",
        "host": "kshww2.thueringen.de",
        "version": "v1.1"
    },
    "thing": {
        "id": 187,
        "name": "Rothenstein",
        "description": "Abflusspegel",
        "properties": {
            "creation_date": "2024-11-27T17:10:38.135601",
            "gewaesser": "Saale",
            "station_id": 570280,
            "type": "TH_PEGEL"
        }
    },
    "location": {
        "type": "Point",
        "coordinates": [11.605473698714054, 50.847739111118464]
    },
    "locations": [
        {
            "id": 187,
            "name": "570280",
            "description": "Location of station 570280",
            "encodingType": "application/geo+json",
            "properties": {},
            "location": {
                "type": "Point",
                "coordinates": [11.605473698714054, 50.847739111118464]
            }
        }
    ],
    "readings": {
        "W": {
            "id": 1024241985,
            "result": 94,
            "time": "2026-11-18T05:00:00Z",
            "phenomenonTime": "2026-11-18T05:00:00Z",
            "unit": { "name": "Centimeter", "symbol": "cm", "definition": "" },
            "datastream": {
                "id": 7668,
                "name": "W@Rothenstein/Saale",
                "description": "",
                "observationType": "",
                "properties": {
                    "measurement_start": "2000-12-31T23:15:00Z",
                    "parameter": "W",
                    "station_id": 570280,
                    "type": "TH_PEGELDATEN_15MIN"
                },
                "phenomenonTime": "2025-11-17T11:00:00Z/2026-11-18T10:45:00Z",
                "observedArea": { "type": "Point", "coordinates": [11.605473699, 50.847739111] }
            },
            "observedProperty": { "id": 71, "name": "W", "definition": "", "description": "W", "properties": {} },
            "sensor": { "id": 73, "name": "W", "description": "Generic W sensor", "encodingType": "", "metadata": "" }
        }
    }
}
```

The discharge at the same instant is a record of its own, holding `readings.Q` and no `readings.W`. A record never holds a water level and a discharge together, because each is an Observation of its own Datastream. Values a server measures together in one Observation belong to a MultiDatastream, which [example 49](../49-sensorthings-multidatastreams/example.md) reads.

## The same mapping

The mapping is [example 43](../43-sensorthings-latest-read/example.md)'s, unchanged. Its guards already leave out an attribute when the record lacks its reading:

- `waterLevel` is guarded twice: `{% if readings.W %}{% if readings.W.result is defined and readings.W.result >= 0 %}...{% endif %}{% endif %}`. The outer test leaves it out of a discharge record. The inner one leaves it out when the server served the result as `null`, where dividing by 100 would fail. It also leaves out a negative reading, which the model does not allow. Testing `readings.W.result` alone would fail on a record without `readings.W`.
- `flow` carries the same two guards over `readings.Q`, so it is left out of every water-level record.

## Series output

```json5
output: {
    target: "file",
    directory: "out",
    context: "default",
    temporal: {
        representation: "series",
    },
    validation: {
        mode: "fail-when-schema",
    },
},
```

Without a `temporal` setting, output is current state: one entity per id, holding each attribute from the record with its latest `observedAt`. That suits a latest read. Here it would keep the last of Rothenstein's 24 water levels and drop the other 23, so Cassiopeia refuses the run before it sends any request:

```text
× The SensorThings input `https://kshww2.thueringen.de/FROST-Server/v1.1` makes a `range` read, one record per Observation, so it runs only into series output, set by output.temporal.representation: "series" or --temporal-representation series: current-state output would keep one reading of each Thing
```

With `representation: "series"`, Cassiopeia folds every record of one entity id into one entity. Each attribute that carries an `observedAt` keeps every reading as an instance, oldest first. An attribute without an `observedAt` is kept once, from the entity's first record. Here that is `dateObserved`, which `WaterObserved` requires. It holds the time of the gauge's first record, the oldest reading of the range. `name`, `description` and `location` repeat on every record and are also kept once.

## Run it

```bash
cassiopeia map \
    --manifest manifest.json5
```

The same run in a container mounts this directory at `/data` and makes it the working directory, so the paths do not change. For Podman, replace `docker` with `podman` and drop the `--user` line: rootless Podman already maps the container's root to your user.

```bash
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "$PWD:/data" \
    --volume "$HOME/.cache/cassiopeia-examples/schemas:/var/lib/cassiopeia/schemas" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.3.0 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner runs the mapping and checks the output in one step:

```bash
cargo run -- run 44
cargo run -- run 44 --runtime docker
```

When this was written, the run read 4,449 Observations into 94 entities. The other gauges had no reading in the six hours: some upload once a day, and some have stopped.

## Read the result

Rothenstein's entity holds 24 water levels and 24 discharges, a quarter of an hour apart. The arrays are cut to their first two instances here:

```json
{
    "@context": "https://raw.githubusercontent.com/smart-data-models/dataModel.Environment/master/context.jsonld",
    "id": "urn:ngsi-ld:WaterObserved:thueringen-570280",
    "type": "WaterObserved",
    "name": {
        "type": "Property",
        "value": "Rothenstein"
    },
    "description": {
        "type": "Property",
        "value": "Gauge on the Saale"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                11.605473698714054,
                50.847739111118464
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-18T05:00:00Z",
        "valueType": "DateTime"
    },
    "flow": [
        {
            "type": "Property",
            "value": 15.5,
            "observedAt": "2026-11-18T05:00:00Z",
            "unitCode": "MQS"
        },
        {
            "type": "Property",
            "value": 15.9,
            "observedAt": "2026-11-18T05:15:00Z",
            "unitCode": "MQS"
        }
    ],
    "waterLevel": [
        {
            "type": "Property",
            "value": 0.94,
            "observedAt": "2026-11-18T05:00:00Z",
            "unitCode": "MTR"
        },
        {
            "type": "Property",
            "value": 0.95,
            "observedAt": "2026-11-18T05:15:00Z",
            "unitCode": "MTR"
        }
    ]
}
```

An attribute with several instances is written as an array, and each instance has its own `observedAt`. This is how NGSI-LD writes the temporal evolution of an entity (ETSI GS CIM 009 v1.9.1, clause 4.5.5 and the EntityTemporal of clause 5.2.20). The last instance of each array is the reading that [example 43](../43-sensorthings-latest-read/example.md)'s latest read wrote.

Each run reads its own six hours and knows nothing of the run before it. Run this every hour and each reading is read six times. [Example 46](../46-sensorthings-incremental-broker/example.md) reads each reading once instead, by keeping a cursor between runs.
