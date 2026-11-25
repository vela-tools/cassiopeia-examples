# Reading SensorThings MultiDatastreams

Every data stream so far was a Datastream: one property, one value per Observation. The MultiDatastream extension of the SensorThings API (OGC 18-088, section 12) groups properties measured together into one data stream, whose Observations each hold an array of values. This example reads a weather station that serves wind speed and gust as one MultiDatastream, next to six ordinary Datastreams. It writes the station as one `WeatherObserved` with a series for each reading. Each value of a MultiDatastream Observation becomes its own reading, and values measured together stay in one record.

## Get the data

[Secure Dimensions](https://www.secure-dimensions.de/) runs a test server for STAplus, an extension of the SensorThings API data model built on FROST-Server, at `https://citiobs.demo.secure-dimensions.de/staplustest/v1.1`. It is one of the few public servers whose MultiDatastreams hold Observations. Because it is a test server, it also holds entities made to try the extension out, next to real stations, and it may change or go away.

The station is Thing 116, an FT-0310 weather station at Spitzingsee in Bavaria, which reported until January 2024. Four Things on the server share its name, so the example selects it by id. STAplus serves a licence with each data stream, and this station's MultiDatastream and Datastreams are licensed [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).

```bash
cassiopeia sensorthings readings https://citiobs.demo.secure-dimensions.de/staplustest/v1.1 \
    --datastreams "Thing/id eq 116" \
    --multi-datastreams "id eq 4"
```

```text
reading  unit  unit code  data streams  multi data streams  things with repeats
RH       %     P1         1             0                   0
UV             unknown    1             0                   0
light    LUX   LUX        1             0                   0
rain     mm    MMT        1             0                   0
temp     C     CEL        1             0                   0
wdir     deg   DD         1             0                   0
wgust    m/s   MTS        0             1                   0
wspeed   m/s   MTS        0             1                   0
```

`wspeed` and `wgust` are counted under `multi data streams`: they are the two values of MultiDatastream 4, "Wind speed avg". This request fetches two of its Observations. Each `result` is an array with one value per property:

```bash
curl --globoff "https://citiobs.demo.secure-dimensions.de/staplustest/v1.1/MultiDatastreams(4)/Observations?\$top=2&\$orderby=phenomenonTime%20asc&\$select=id,phenomenonTime,result"
```

## Get the schema

```bash
cassiopeia sdm download
```

## Selecting MultiDatastreams

```json5
sensorthings: {
    datastreams: "Thing/id eq 116",
    multiDatastreams: "id eq 4",
    read: { range: { from: "2024-01-19T18:00:00Z", to: "2024-01-19T20:00:00Z" } },
},
```

`datastreams` selects Datastreams only. `multiDatastreams` selects MultiDatastreams: `false` (the default) for none, `true` for all, or a filter of its own. A MultiDatastream has `ObservedProperties` where a Datastream has an `ObservedProperty`, so a filter written for one kind may not work for the other. That is why each kind takes its own filter. Without `multiDatastreams`, a read sends no request to the MultiDatastreams collection. [Example 47](../47-sensorthings-station-inventory/example.md) shows the warning a run gives when the server holds some.

The range is fixed, from 18:00 to 20:00 on 2024-01-19, so every run reads the same Observations. The read lists the six Datastreams and the MultiDatastream. It then requests the Observations of each Datastream, and last those of the MultiDatastream, sending the request of [example 44](../44-sensorthings-range-read/example.md) to `MultiDatastreams(4)/Observations`. This server lists `ObservedProperties` 1825 and 1826 in the same order as the values in its results. Before mapping a value by its name, check that order with a latest read or the `readings` table.

## The record shape

A range read gives one record per Observation. An Observation of a Datastream gives one reading. An Observation of the MultiDatastream gives one reading per value, keyed by the name of the ObservedProperty at its position, so wind speed and gust stay in one record. The readings of one such record, cut to the members that differ from a Datastream's:

```json
"readings": {
    "wspeed": {
        "id": 307234,
        "result": 1.2,
        "time": "2024-01-19T18:17:06Z",
        "unit": { "name": "Meters per second", "symbol": "m/s", "definition": "https://qudt.org/vocab/unit/M-PER-SEC" },
        "multiDatastream": {
            "id": 4,
            "name": "Wind speed avg",
            "observationType": "http://www.opengis.net/def/observationType/OGC-OM/2.0/OM_Measurement",
            "position": 1
        },
        "observedProperty": { "id": 1825, "name": "wspeed", "definition": "http://vocab.nerc.ac.uk/standard_name/wind_speed/", "description": "Wind speed" }
    },
    "wgust": {
        "id": 307234,
        "result": 1.7,
        "time": "2024-01-19T18:17:06Z",
        "unit": { "name": "Meters per second", "symbol": "m/s", "definition": "https://qudt.org/vocab/unit/M-PER-SEC" },
        "multiDatastream": {
            "id": 4,
            "name": "Wind speed avg",
            "observationType": "http://www.opengis.net/def/observationType/OGC-OM/2.0/OM_Measurement",
            "position": 2
        },
        "observedProperty": { "id": 1826, "name": "wgust" }
    }
}
```

Both readings carry the Observation's `id` and `time`, since they are one Observation. Each has its own `result` (the value at its position), its own `unit` from the MultiDatastream's `unitOfMeasurements`, and its own `observedProperty`. In place of `datastream`, a reading of a MultiDatastream has `multiDatastream`, with the value's `observationType` from `multiObservationDataTypes` and its `position`, counted from 1. Everything else (the Thing, its Locations, the Sensor) is the same as for a Datastream.

A record of a Datastream Observation, such as the air temperature at the same instant, holds one reading, `temp`, with a `datastream` member as in the earlier examples. The mapping reads both kinds the same way, by key.

## The same mapping for both kinds

```json5
windSpeed: {
    source: "{{ readings.wspeed.result }}",
    transformation: "float",
    properties: {
        unitCode: {
            source: "{% if readings.wspeed %}{{ readings.wspeed.unit | cefact_unit }}{% endif %}",
        },
        observedAt: {
            source: "{{ readings.wspeed.time }}",
            transformation: "datetime",
        },
    },
},
```

`gustSpeed` reads `readings.wgust` the same way. `windDirection`, `temperature`, `precipitation` and `illuminance` read the Datastreams' `wdir`, `temp`, `rain` and `light`. Nothing in the mapping says which reading came from a MultiDatastream. `relativeHumidity` divides `RH` by 100 and writes `C62`, the UN/CEFACT code for one, because the model gives it as a fraction from 0 to 1 and the server serves percent. `UV` has no unit the model's `uVIndexMax` would take, so it is left out. `dateObserved`, which `WeatherObserved` requires, takes the time of a record's first reading. It has no `observedAt`, so series output keeps it once, from the entity's first record. The identity uses the server's id, `staplus-{{ thing.id }}`, because four Things share the name.

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
cargo run -- run 49
cargo run -- run 49 --runtime docker
```

The read takes about two seconds. It gives 7,014 records: 1,002 from the MultiDatastream, with two readings each, and 6,012 from the Datastreams, with one each. All of them go into one entity in `out/WeatherObserved.jsonld`.

## Read the result

The entity holds seven series, each of 1,002 instances. Its arrays are cut to their first two instances here:

```json
{
    "@context": "https://raw.githubusercontent.com/smart-data-models/dataModel.Weather/master/context.jsonld",
    "id": "urn:ngsi-ld:WeatherObserved:staplus-116",
    "type": "WeatherObserved",
    "name": {
        "type": "Property",
        "value": "FT-0310 Weather Station"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                11.88533,
                47.659664
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2024-01-19T18:17:06Z",
        "valueType": "DateTime"
    },
    "windSpeed": [
        {
            "type": "Property",
            "value": 1.2,
            "observedAt": "2024-01-19T18:17:06Z",
            "unitCode": "MTS"
        },
        {
            "type": "Property",
            "value": 1.2,
            "observedAt": "2024-01-19T18:17:06Z",
            "unitCode": "MTS"
        }
    ],
    "gustSpeed": [
        {
            "type": "Property",
            "value": 1.7,
            "observedAt": "2024-01-19T18:17:06Z",
            "unitCode": "MTS"
        },
        {
            "type": "Property",
            "value": 1.7,
            "observedAt": "2024-01-19T18:17:06Z",
            "unitCode": "MTS"
        }
    ]
}
```

`windSpeed` and `gustSpeed` share their instants because each pair of values is one Observation. The other series are left out of the quote. The first two instances of each series are the same reading twice. The station published every reading more than once, as separate Observations with their own ids and the same `phenomenonTime` and `result`: Observations 307234 and 307241 both hold `[1.2, 1.7]` at 18:17:06. Each series has 1,002 instances at 334 distinct instants. Cassiopeia writes each Observation it reads, and instances with the same `observedAt` keep the order of their Observations, so the output holds the repeats exactly as the server does. A broker's temporal API would store them the same way. The run has no way to tell a repeated upload from two readings that happen to agree, so it keeps both.

`precipitation` stays near 19.5 mm across the two hours because the station serves its rain counter rather than the rain of each interval. `wgust`'s ObservedProperty points to the definition of a wind direction, `wind_gust_from_direction`, but its values are speeds. The mapping goes by the values and their unit, `m/s`.
