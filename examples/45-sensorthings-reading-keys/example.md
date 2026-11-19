# Choosing among repeated SensorThings readings

[Example 43](../43-sensorthings-latest-read/example.md) selected one Datastream per property and gauge, so each reading had a plain key: `readings.W`, `readings.Q`. Drop that narrowing and most gauges of the Thuringian network serve each property twice, once as a daily value and once every 15 minutes. A record then holds two readings of the same name, and only one of them can keep it. This example shows which one keeps the plain key, and why `readings.W` then misleads. Its mapping runs over many gauges and picks each reading by its Datastream, not by its key. The run writes one `WaterObserved` per gauge, holding its finest water level and discharge.

## Get the data

The source is the FROST-Server of the Thuringian State Office for Environment, Mining and Nature Conservation (TLUBN) at `https://kshww2.thueringen.de/FROST-Server/v1.1`, as in [example 43](../43-sensorthings-latest-read/example.md). TLUBN names itself as the owner of the data and states no licence for the service. This time the selection is every water level and every discharge, whatever its resolution:

```bash
cassiopeia sensorthings readings https://kshww2.thueringen.de/FROST-Server/v1.1 \
    --datastreams "ObservedProperty/name eq 'W' or ObservedProperty/name eq 'Q'"
```

```text
reading  unit  unit code  data streams  multi data streams  things with repeats
Q        m³/s  MQS        277           0                   123
W        cm    CMT        304           0                   125
```

`things with repeats` counts the gauges that hold more than one reading of a name: 125 gauges serve two water levels, and 123 two discharges. The type in each Datastream's `properties` says which is which: `TH_PEGELDATEN_1D` for a daily value, `TH_PEGELDATEN_15MIN` for a 15-minute reading, and `TH_PEGELDATEN_LORA` for the water level of 21 small gauges that report over LoRaWAN.

## Get the schema

```bash
cassiopeia sdm download
```

## The record shape

A latest record holds one reading per Datastream, keyed by its ObservedProperty's name. When two readings of one Thing share a name, the reading of the Datastream with the lowest id keeps it, and every other one is keyed `<name>@<Datastream id>`. The Rothenstein gauge's record, with each reading cut to the members that tell them apart:

```json
{
    "thing": { "id": 187, "name": "Rothenstein" },
    "readings": {
        "Q": {
            "result": 16.1,
            "time": "2026-11-18T00:00:00Z",
            "datastream": { "id": 7185, "properties": { "type": "TH_PEGELDATEN_1D" } }
        },
        "W": {
            "result": 95.5,
            "time": "2026-11-18T00:00:00Z",
            "datastream": { "id": 7339, "properties": { "type": "TH_PEGELDATEN_1D" } }
        },
        "Q@7510": {
            "result": 19.3,
            "time": "2026-11-18T10:15:00Z",
            "datastream": { "id": 7510, "properties": { "type": "TH_PEGELDATEN_15MIN" } }
        },
        "W@7668": {
            "result": 103,
            "time": "2026-11-18T10:30:00Z",
            "datastream": { "id": 7668, "properties": { "type": "TH_PEGELDATEN_15MIN" } }
        }
    }
}
```

The keys come from the Datastreams alone, so a reading has the same key in every read: Rothenstein's 15-minute water level is always `W@7668`. The run reports the generated keys once and names the first three:

```text
ingest-readings-renamed  248  In `https://kshww2.thueringen.de/FROST-Server/v1.1`, readings of one Thing observe the same property, so 248 readings are read by generated keys: Thing 577 reads Datastream 7431 as `readings['Q@7431']`; Thing 577 reads Datastream 7593 as `readings['W@7593']`; Thing 568 reads Datastream 7436 as `readings['Q@7436']`; and 245 more
```

## The trap

`readings.W` holds the reading of the Datastream with the lowest id. The server created the daily Datastreams first, so on most gauges `readings.W` is the daily value. Rothenstein's is stamped midnight, while its current water level is `W@7668`, ten and a half hours newer. Run on this selection, [example 43](../43-sensorthings-latest-read/example.md)'s mapping would write daily values, and nothing in the output would say so. `W` does not mean the same on every gauge either. Two gauges serve a 15-minute water level and no daily one, so there `W` is the 15-minute reading. The LoRa gauges serve one water level only, so on them `W` is the LoRa reading.

There are three ways out. The first is to narrow the selection when you need one resolution. [Example 43](../43-sensorthings-latest-read/example.md)'s filter, `properties/type eq 'TH_PEGELDATEN_15MIN'`, leaves one water level per gauge, so `readings.W` is the 15-minute reading and the warning goes away. That is the right answer whenever a filter can say what you want.

The second is to read one Thing's reading by its key. A key that is not a plain name is read with brackets: `{{ readings['W@7668'].result }}`. That works for a mapping of Rothenstein alone. A mapping that runs over many gauges cannot name such keys, because the id differs from gauge to gauge.

The third is to pick by what the Datastream is. A mapping over many gauges can loop over the readings and choose by the members every reading carries, whatever its key. This example does that.

## Picking a reading by its Datastream

```json5
waterLevel: {
    source: "{% set_global value = '' %}{% for key, reading in readings %}{% if reading.observedProperty.name == 'W' and reading.result is defined and reading.result >= 0 %}{% if value == '' or reading.datastream.properties.type != 'TH_PEGELDATEN_1D' %}{% set_global value = reading.result / 100 %}{% endif %}{% endif %}{% endfor %}{{ value }}",
    transformation: "float",
    properties: {
        unitCode: {
            source: "MTR",
        },
        observedAt: {
            source: "{% set_global time = '' %}{% for key, reading in readings %}{% if reading.observedProperty.name == 'W' and reading.result is defined and reading.result >= 0 %}{% if time == '' or reading.datastream.properties.type != 'TH_PEGELDATEN_1D' %}{% set_global time = reading.time %}{% endif %}{% endif %}{% endfor %}{{ time }}",
            transformation: "datetime",
        },
    },
}
```

`{% for key, reading in readings %}` visits every reading of the record. The outer test keeps a water level with a usable result: one the server served, and not below the model's minimum of 0. The inner test takes the first such reading, and lets any reading that is not a daily value replace it. A gauge with a 15-minute or a LoRa reading therefore gives that reading, and a gauge that serves daily values only gives its daily value. Tera's plain `set` would not carry the choice out of the loop, so the mapping uses `set_global`. `observedAt` repeats the same choice, so the time written is the time of the value written. `flow` does the same for `Q`.

`dateObserved`, which `WaterObserved` requires, takes the newest time among all of the gauge's readings, by the same kind of loop. Every reading's `time` is a UTC instant in the same format, so the times compare correctly as text.

## A gauge without readings

One gauge, set up a few days before this was written, had Datastreams and no Observation in any of them. A latest read still gives it a record, without `readings`. An entity built from that record would have no `dateObserved` and would fail validation, so the identity is guarded:

```json5
identity: {
    entityName: "{% if readings %}thueringen-{{ thing.properties.station_id }}{% endif %}",
}
```

A record without readings resolves to no identity, so the run skips it and counts it once under `expander-id-absent`.

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
    ghcr.io/vela-tools/cassiopeia:v1.2.0 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner runs the mapping and checks the output in one step:

```bash
cargo run -- run 45
cargo run -- run 45 --runtime docker
```

The read lists the selected Datastreams, 581 when this was written, in six pages of 100 and takes a little over a minute. It writes one entity per gauge with a reading, 180 at the same time.

## Read the result

Rothenstein's entity holds its 15-minute readings, not the daily values its plain keys hold:

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
        "observedAt": "2026-11-18T10:45:00Z",
        "unitCode": "MQS"
    }
}
```

A LoRa gauge, Apfelstädtaue, serves one water level and no discharge, so its entity has no `flow`. Its readings are stamped to the second, as they arrive:

```json
{
    "@context": "https://raw.githubusercontent.com/smart-data-models/dataModel.Environment/master/context.jsonld",
    "id": "urn:ngsi-ld:WaterObserved:thueringen-58778093",
    "type": "WaterObserved",
    "name": {
        "type": "Property",
        "value": "Apfelstädtaue"
    },
    "description": {
        "type": "Property",
        "value": "Gauge on the Apfelstädt"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                10.817949150611382,
                50.906679656236676
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-18T10:54:32Z",
        "valueType": "DateTime"
    },
    "waterLevel": {
        "type": "Property",
        "value": 0.003072779553375244,
        "observedAt": "2026-11-18T10:54:32Z",
        "unitCode": "MTR"
    }
}
```

Its water level of three millimetres is what the gauge reports near its zero. At other times it reports a little below zero, and the mapping leaves those readings out. A gauge that serves daily values only, such as one closed decades ago, gets its daily value. When the server holds a gauge's last daily value with a `null` result, the gauge gets an entity with its `dateObserved`, `name` and `location` and no `waterLevel`.

The published schema gives `waterLevel` one number, so one `WaterObserved` cannot hold a gauge's values at both resolutions side by side. NGSI-LD itself can hold both, as two instances of `waterLevel` with a `datasetId` each (ETSI GS CIM 009 v1.9.1, clause 4.5.5). [Example 21](../21-json-dataset-id/example.md) keeps three forecast models apart that way. An entity written like that fails validation against the published `WaterObserved` schema.
