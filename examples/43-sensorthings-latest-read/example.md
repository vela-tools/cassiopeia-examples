# Reading the newest readings from a SensorThings server

The examples before this one map files. This one maps a server, an [OGC SensorThings API](https://www.ogc.org/standards/sensorthings/) service that publishes the river gauges of the German state of Thuringia. A latest read asks the server for the newest reading of each data stream. Cassiopeia joins each reading with the gauge that made it, so the mapping sees one record per gauge, and the run writes one `WaterObserved` entity per gauge with its current water level and discharge. The next three examples read the same kind of server in other ways: [example 44](../44-sensorthings-range-read/example.md) reads a time range, [example 45](../45-sensorthings-reading-keys/example.md) reads several data streams of one property, and [example 46](../46-sensorthings-incremental-broker/example.md) reads only what is new, on a schedule.

## Get the data

The Thuringian State Office for Environment, Mining and Nature Conservation (TLUBN) runs a [FROST-Server](https://github.com/FraunhoferIOSB/FROST-Server) at `https://kshww2.thueringen.de/FROST-Server/v1.1`, the service root that every resource of the server is addressed under. It names itself as the owner of the data and states no licence for the service, so credit the data to TLUBN and check its terms before you reuse it.

There is no file to download. A SensorThings server organises its data as entities: a Thing is a gauge, its Location is where it stands, and each Datastream is one series of Observations of one ObservedProperty, such as the water level, made by one Sensor. Each gauge here serves its water level, `W` in centimetres, and its discharge, `Q` in cubic metres per second, twice: as daily means and as 15-minute readings. The type in each Datastream's `properties` tells them apart, and this example reads the 15-minute ones.

Before writing a mapping, list what the selection offers:

```bash
cassiopeia sensorthings readings https://kshww2.thueringen.de/FROST-Server/v1.1 \
    --datastreams "properties/type eq 'TH_PEGELDATEN_15MIN'"
```

```text
reading  unit  unit code  data streams  multi data streams  things with repeats
Q        m³/s  MQS        123           0                   0
W        cm    CMT        127           0                   0
```

Each row is one ObservedProperty name, which the mapping reads as `readings.<name>`. 127 gauges serve a 15-minute water level and 123 a 15-minute discharge. `unit code` is what the `cefact_unit` filter turns each unit into. `things with repeats` counts the gauges that serve one name more than once, the subject of [example 45](../45-sensorthings-reading-keys/example.md). `--datastreams` is an OData `$filter` over Datastreams. The server applies it, and the manifest below uses the same filter, so the command and the run read the same selection.

This request fetches one Datastream, the 15-minute water level of the Rothenstein gauge on the Saale. The server answers with the nested JSON that a mapping would otherwise have to take apart:

```bash
curl --globoff "https://kshww2.thueringen.de/FROST-Server/v1.1/Datastreams(7668)?\$expand=Thing/Locations,ObservedProperty,Sensor,Observations(\$orderby=phenomenonTime%20desc;\$top=1)"
```

## Get the schema

`WaterObserved` is a published Smart Data Model. Download the catalog once, so both validation and the model's `@context` can be resolved:

```bash
cassiopeia sdm download
```

## The record shape

A latest read sends one request for the selected Datastreams and follows the server's pages. Each Datastream comes with its Thing, the Thing's Locations, its ObservedProperty, its Sensor and its newest Observation. Cassiopeia then gives the mapping one record per Thing. This is the record for the Rothenstein gauge:

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
        "Q": {
            "id": 1024251467,
            "result": 19.3,
            "time": "2026-11-18T10:30:00Z",
            "phenomenonTime": "2026-11-18T10:30:00Z",
            "unit": { "name": "Kubikmeter pro Sekunde", "symbol": "m³/s", "definition": "" },
            "datastream": {
                "id": 7510,
                "name": "Q@Rothenstein/Saale",
                "description": "",
                "observationType": "",
                "properties": {
                    "measurement_start": "2000-12-31T23:00:00Z",
                    "parameter": "Q",
                    "station_id": 570280,
                    "type": "TH_PEGELDATEN_15MIN"
                },
                "phenomenonTime": "2025-11-17T11:00:00Z/2026-11-18T10:30:00Z",
                "observedArea": { "type": "Point", "coordinates": [11.605473699, 50.847739111] }
            },
            "observedProperty": { "id": 64, "name": "Q", "definition": "", "description": "Q", "properties": {} },
            "sensor": { "id": 64, "name": "Q", "description": "Generic Q sensor", "encodingType": "", "metadata": "" }
        },
        "W": {
            "id": 1024251595,
            "result": 103,
            "time": "2026-11-18T10:45:00Z",
            "phenomenonTime": "2026-11-18T10:45:00Z",
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

Cassiopeia builds this record from the server's answer, and the record has the same shape whatever JSON a SensorThings server sends. `server` names the service root. `thing` holds the Thing's members. `location` is the geometry of the Thing's GeoJSON Location, and `locations` keeps every Location as served, which [example 48](../48-sensorthings-multiple-locations/example.md) needs. `readings` holds one entry per Datastream, keyed by its ObservedProperty's name. Each entry carries the Observation's `result` and `phenomenonTime`, the Datastream's `unit`, and the Datastream, ObservedProperty and Sensor the reading came from. `time` is the instant that `phenomenonTime` stands for, in UTC.

The mapping reads `thing.name`, `thing.properties.station_id`, `thing.properties.gewaesser`, `location`, and the `result`, `time` and `unit` of `readings.W` and `readings.Q`. It ignores everything else.

## One entity per gauge

```json5
identity: {
    entityName: "thueringen-{{ thing.properties.station_id }}",
}
```

`thing.id` is the server's own key and would work too. The state's gauge number is the better choice: the gauge is known by it elsewhere, and it survives a reload of the server. The name would not work, because a Thing's name here is the place, and one place can have two gauges. Rothenstein's entity is `urn:ngsi-ld:WaterObserved:thueringen-570280`.

## Units and times

`WaterObserved` defines `waterLevel` in metres, and the server serves centimetres, so the mapping divides by 100 and writes `MTR`, the UN/CEFACT code for metre:

```json5
waterLevel: {
    source: "{% if readings.W %}{% if readings.W.result is defined and readings.W.result >= 0 %}{{ readings.W.result / 100 }}{% endif %}{% endif %}",
    transformation: "float",
    properties: {
        unitCode: {
            source: "MTR",
        },
        observedAt: {
            source: "{{ readings.W.time }}",
            transformation: "datetime",
        },
    },
}
```

Discharge keeps the unit the server serves. The `cefact_unit` filter turns the Datastream's `unit` into its UN/CEFACT code, `MQS` for cubic metre per second, which is the `unit code` column of the readings table:

```json5
flow: {
    source: "{% if readings.Q %}{% if readings.Q.result is defined and readings.Q.result >= 0 %}{{ readings.Q.result }}{% endif %}{% endif %}",
    transformation: "float",
    properties: {
        unitCode: {
            source: "{% if readings.Q %}{{ readings.Q.unit | cefact_unit }}{% endif %}",
        },
        observedAt: {
            source: "{{ readings.Q.time }}",
            transformation: "datetime",
        },
    },
}
```

A filter fails on a missing field, and a qualifier is resolved even when its attribute is not. The `if` around `cefact_unit` keeps a gauge without a discharge from failing its `unitCode`. The next section covers the guards on the values. Each reading's `time` becomes its attribute's `observedAt`, because the water level and the discharge of one gauge are not always taken at the same instant: Rothenstein's discharge above is a quarter of an hour older than its water level. `dateObserved`, which `WaterObserved` requires, takes the water level's time, and the discharge's when a gauge serves no water level.

## Readings without a usable result

The server serves some newest Observations with `"result": null`. Cassiopeia leaves a member served as `null` out of the record, so such a gauge still has a reading, only without `result`. When this was written, 23 of the 127 water levels looked like that. A plain reference such as `{{ readings.W.result }}` resolves to nothing there, and the attribute is left out. Arithmetic is stricter: `readings.W.result / 100` fails on a missing field, and the run would report every such gauge. The inner guard tests `is defined` rather than the value itself, because `{% if readings.W.result %}` would also drop a water level of exactly `0`. The outer guard covers a gauge with no water level reading at all, where even the `is defined` test fails. In the range read of [example 44](../44-sensorthings-range-read/example.md), every record is missing one of the two readings.

`WaterObserved` also gives `waterLevel` and `flow` a minimum of 0. A gauge reads relative to its own zero, and a gauge near that zero can report a value a little below it. Such a value fails validation, and under `fail-when-schema` a single invalid entity stops the run. The inner guard therefore also leaves out a negative value, and the entity is written without that attribute. `flow` carries the same two guards as `waterLevel`, for the same reasons.

## The manifest

The `sensorthings` block exists only in a manifest, so every SensorThings example runs one:

```json5
{
    version: "v1",
    inputs: [
        {
            source: "https://kshww2.thueringen.de/FROST-Server/v1.1",
            format: "sensorthings",
            mapping: "water-observed.json5",
            sensorthings: {
                datastreams: "properties/type eq 'TH_PEGELDATEN_15MIN'",
                read: "latest",
                pageSize: 100,
                delay: "200ms",
            },
        },
    ],
    output: {
        target: "file",
        directory: "out",
        context: "default",
        validation: {
            mode: "fail-when-schema",
        },
    },
}
```

- `source` is the service root, and `format: "sensorthings"` reads it as a server.
- `datastreams` is the same filter the `readings` command took. Without it, the read would take every Datastream of the server, the daily ones and those of its groundwater wells included.
- `read: "latest"` reads the newest reading of each selected Datastream. It is the default, written out here because the next examples change it.
- `pageSize` is the `$top` of each request. The server looks up the newest Observation of each Datastream while it builds a page, so a page of the default 1000, which holds all 250 Datastreams, takes about 30 seconds. That is as long as Cassiopeia waits for an answer by default, and the read fails with `error decoding response body`. A page of 100 answers in about 10 seconds, and the read follows three pages.
- `delay` spaces the requests, to spare a public service.

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
    ghcr.io/vela-tools/cassiopeia:v1.3.1 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner runs the mapping and checks the output in one step:

```bash
cargo run -- run 43
cargo run -- run 43 --runtime docker
```

The read takes about 25 seconds and writes `out/WaterObserved.jsonld`, with one entity per gauge (127 when this was written), each carrying the `@context` of the model's repository.

## Read the result

Rothenstein's entity holds the two readings of its record:

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
        "value": "2026-11-18T10:45:00Z",
        "valueType": "DateTime"
    },
    "waterLevel": {
        "type": "Property",
        "value": 1.03,
        "observedAt": "2026-11-18T10:45:00Z",
        "unitCode": "MTR"
    },
    "flow": {
        "type": "Property",
        "value": 19.3,
        "observedAt": "2026-11-18T10:30:00Z",
        "unitCode": "MQS"
    }
}
```

`observedAt` says when each reading was observed (ETSI GS CIM 009 v1.9.1, clause 4.8), and `unitCode` beside it gives its unit (clause 4.5.2). Run the read again a quarter of an hour later and both values will have moved on.

"Newest" means the Observation with the latest `phenomenonTime`, as the server holds it. A gauge that stopped reporting keeps an old reading, and some entities carry a `dateObserved` from 2025. One gauge, Langendembach (`urn:ngsi-ld:WaterObserved:thueringen-572450`), holds an Observation stamped `2251-01-04T17:15:00Z`, a typing error in the source. A latest read takes it as the gauge's newest reading and writes it unchanged. The [incremental read in example 46](../46-sensorthings-incremental-broker/example.md) never picks up such a reading, because it reads only Observations stamped before each cycle starts. [Source formats](https://vela-tools.github.io/cassiopeia/docs/guides/source-formats#sensorthings-api) describes the requests, the record, and how servers differ.
