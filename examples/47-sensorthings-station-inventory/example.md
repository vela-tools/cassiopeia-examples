# Listing every station of a SensorThings server

The examples so far mapped readings. This one maps the stations themselves: every Thing of the British Geological Survey's SensorThings server becomes a `Device`, with where it stands, what it measures, when it started and when it last reported. A latest read finds Things through their data streams, so a station with no data stream at all would be missing from such an inventory. `things: true` also lists every Thing directly, so a Thing without data streams gives a record of its own, without readings.

## Get the data

The [British Geological Survey](https://www.bgs.ac.uk/) runs a FROST-Server at `https://sensors.bgs.ac.uk/FROST-Server/v1.1` for its groundwater, soil gas, weather and geothermal monitoring, much of it at the UK Geoenergy Observatories in Glasgow and Cheshire. Each Thing and Datastream states its terms in its `data_usage` property: the [Open Government Licence](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/), with the acknowledgement "Contains UKRI materials ©UKRI [year]". When this was written, the server held 176 Things, 494 Datastreams and 42 MultiDatastreams. The example reads all of it, without a filter:

```bash
cassiopeia sensorthings readings https://sensors.bgs.ac.uk/FROST-Server/v1.1
```

The table is longer than in the earlier examples. These are some of its rows:

```text
reading                            unit       unit code  data streams  multi data streams  things with repeats
Barometric Pressure                mbar       MBR        5             0                   0
Carbon Dioxide                     %          P1         2             0                   0
Conductivity                       uS/cm      G42        14            0                   0
DTS Depth                          m          MTR        0             30                  0
DTS Temperature                    C          COU        0             30                  0
Event                              -          unknown    166           0                   0
Head of Water                      m          MTR        36            0                   0
Salinity                           PSU        unknown    14            0                   0
Water Level maOD                   m          MTR        14            0                   0
Water Temperature                  C          COU        121           0                   0
```

Two things in the table need a second look. `C` for degrees Celsius resolves to `COU`, the UN/CEFACT code for coulomb, because `C` is the coulomb's symbol. A mapping that wrote these readings would give `unitCode: "CEL"` as a constant rather than trust `cefact_unit` with a bare `C`. The `DTS` rows count MultiDatastreams only, which the next sections come back to. This example maps no reading, so neither matters here.

## Get the schema

```bash
cassiopeia sdm download
```

## The record shape

```json5
sensorthings: {
    read: "latest",
    things: true,
},
```

A latest read sends its request for the Datastreams, each with its newest Observation, and gives one record per Thing those Datastreams name. With `things: true` it then lists every Thing of the server, each with its Locations:

```text
{root}/Things?$top=1000&$expand=Locations&$orderby=id%20asc
```

A Thing the Datastreams already named gives its record once. Every other Thing gives a record after them, with `server`, `thing`, `location` and `locations`, and no `readings`. The GasClam probe of BGS Site 5 has no data stream and, when this was written, no Location either, so its record is the Thing alone:

```json
{
    "server": {
        "url": "https://sensors.bgs.ac.uk/FROST-Server/v1.1",
        "host": "sensors.bgs.ac.uk",
        "version": "v1.1"
    },
    "thing": {
        "id": 500,
        "name": "BGS Site 5 GasClam",
        "description": "GasClam Probe providing continuous gas and environmental monitoring",
        "properties": {
            "access_restriction": "Unrestricted",
            "arrays": [
                { "id": "GGEM", "name": "UKGEOS Glasgow Environmental Monitoring", "type": "Environmental Monitoring" },
                { "id": "SOILGAS", "name": "Soil Gas Monitoring", "type": "Soil Gas Monitoring" }
            ],
            "category": "Soil Gas Monitoring",
            "data_capture": "NonTelemetered",
            "data_usage": "Available under the Open Government Licence subject to the following acknowledgement accompanying the reproduced UKRI materials \"Contains UKRI materials ©UKRI [year]\"",
            "observation_start_date": "2021-09-01T00:00:00Z",
            "provider": {
                "@id": "http://ror.org/04a7gbp98",
                "name": "British Geological Survey",
                "@type": "sdo:Organization",
                "identifier": { "@type": "sdo:PropertyValue", "value": "ror:04a7gbp98", "propertyID": "http://registry.identifiers.org/registry/ror" }
            },
            "publish_yn": "Y",
            "sen_id": "GGS05_02",
            "serial_no": "000028/09/14",
            "type": "SENSOR"
        }
    }
}
```

A Thing with readings has the same record as in the earlier examples. Each reading also carries the Datastream's `phenomenonTime`, the interval all of its Observations span, and the Sensor that makes them. This is a water temperature logger's reading, cut to those members:

```json
"readings": {
    "Water Temperature": {
        "result": 14.155,
        "time": "2023-08-16T18:30:00Z",
        "datastream": {
            "id": 213,
            "name": "8OB101 Water Temperature",
            "phenomenonTime": "2015-06-19T09:30:00Z/2023-08-16T18:30:00Z"
        },
        "sensor": {
            "id": 15,
            "name": "Solinst Levelogger",
            "description": "Solinst Levelogger",
            "encodingType": "application/pdf",
            "metadata": "https://www.solinst.com/products/data/3001.pdf"
        }
    }
}
```

The mapping reads the Thing's `id`, `name`, `description`, `location`, and the `category`, `serial_no`, `provider.name` and `observation_start_date` of its `properties`, and the `datastream.phenomenonTime` of each reading. It ignores the readings' values and Sensors. A Sensor's `metadata` is often a datasheet, as here, and just as often a placeholder such as `To be confirmed`. Cassiopeia passes it on as served and never fetches it. [Example 51](../51-sensorthings-taiwan-end-to-end/example.md) maps Sensors as Devices of their own.

## One Device per Thing

```json5
identity: {
    entityName: "bgs-{{ thing.id }}",
},
```

BGS renames a Thing as its state changes, adding `(retired)` to a station that stopped, so the server's id is the identity. Most of the mapping copies the Thing's members: `name`, `description`, `location`, `serialNumber` from `serial_no`, `provider` from the provider's name, and `dateFirstUsed` from `observation_start_date`. A member the Thing does not have, such as a serial number on most loggers or the GasClam's Location, is left out of its Device.

`Device` requires `controlledProperty`, a list of values from a fixed set, and a Thing without readings has to have one too. Every Thing carries a monitoring `category`, so the list is chosen by category:

```json5
controlledProperty: {
    source: "{% set category = thing.properties.category %}{% if category == 'Groundwater Monitoring' and 'Water Temperature Logger' in thing.name %}[\"temperature\"]{% elif category == 'Groundwater Monitoring' %}[\"waterLevel\", \"temperature\"]{% elif category == 'DTS Monitoring' or category == 'DTC Monitoring' %}[\"temperature\"]{% elif category == 'Barometer' %}[\"atmosphericPressure\"]{% elif category == 'Weather Station' %}[\"weatherConditions\"]{% elif category == 'Soil Gas Monitoring' or category == 'Atmospheric Monitoring' %}[\"airPollution\"]{% elif category == 'ERT Monitoring' %}[\"conductivity\"]{% endif %}",
    transformation: "array",
},
```

The template writes the list as JSON text, which the `array` transformation reads as an array. The set has no value for soil gas or for ground resistivity, so the gas probes take `airPollution` and the resistivity arrays `conductivity`, the nearest values it has. The published model forces that choice; the server itself says nothing about it. A Thing whose category the lookup does not know would get no `controlledProperty`. It would fail validation and stop the run, and the lookup would then need another line.

## When a station last reported

```json5
dateLastValueReported: {
    source: "{% if readings %}{% set_global latest = '' %}{% for key, reading in readings %}{% if reading.datastream.phenomenonTime %}{% set end = reading.datastream.phenomenonTime | split(pat='/') | last %}{% if latest == '' or end > latest %}{% set_global latest = end %}{% endif %}{% endif %}{% endfor %}{{ latest }}{% endif %}",
    transformation: "datetime",
},
```

The end of a Datastream's `phenomenonTime` is the time of its last Observation, so the latest end over a Thing's readings is when the station last reported. This server also gives a `phenomenonTime` to Datastreams that serve no Observation at all, but a latest read gives no reading for a data stream without a newest Observation, so those never reach the template. The end of an interval is the instant after its `/`. An instant holds no `/`, so `last` returns it whole. The filters fail on a missing field, so the template passes over a Datastream without a `phenomenonTime`. It also passes over a Thing without readings, because a loop over a missing field fails too. No Datastream of this server reaches the present. Most of its stations report in batches or have stopped, and an old `dateLastValueReported` shows that plainly.

## The MultiDatastreams the run does not read

The run reports one warning:

```text
collect-multi-datastreams-unread  1  `https://sensors.bgs.ac.uk/FROST-Server/v1.1` holds MultiDatastreams, which this input reads none of; set `multiDatastreams` to `true`, or to an OData expression that selects some, to read their readings
```

A latest read selects Datastreams, and selects MultiDatastreams only when `multiDatastreams` asks for them. The distributed temperature sensing profiles, `DTS Profile ...`, keep their readings in MultiDatastreams, which measure depth and temperature along a fibre-optic cable together. Without `things: true`, a profile whose only data stream with readings is a MultiDatastream would be missing from the inventory. The warning says that the server holds more than the run reads. When this was written, every one of the 42 MultiDatastreams was empty, so `multiDatastreams: true` would add no readings here. [Example 49](../49-sensorthings-multidatastreams/example.md) reads MultiDatastreams that hold Observations.

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
cargo run -- run 47
cargo run -- run 47 --runtime docker
```

The read takes about six seconds and writes `out/Device.jsonld`. When this was written, the file held 176 Devices, 106 of them from Things without readings. Only seven of those Things have no data stream at all. The other 99 have data streams, but none of them holds an Observation, which on this server is the more common way for a station to have nothing to report.

## Read the result

The GasClam probe's Device holds what its Thing says about it, and nothing about readings:

```json
{
    "@context": "https://raw.githubusercontent.com/smart-data-models/dataModel.Device/master/context.jsonld",
    "id": "urn:ngsi-ld:Device:bgs-500",
    "type": "Device",
    "name": {
        "type": "Property",
        "value": "BGS Site 5 GasClam"
    },
    "description": {
        "type": "Property",
        "value": "GasClam Probe providing continuous gas and environmental monitoring"
    },
    "deviceCategory": {
        "type": "Property",
        "value": [
            "sensor"
        ]
    },
    "controlledProperty": {
        "type": "Property",
        "value": [
            "airPollution"
        ]
    },
    "serialNumber": {
        "type": "Property",
        "value": "000028/09/14"
    },
    "provider": {
        "type": "Property",
        "value": "British Geological Survey"
    },
    "dateFirstUsed": {
        "type": "Property",
        "value": "2021-09-01T00:00:00Z",
        "valueType": "DateTime"
    }
}
```

The water temperature logger's Device has a place and a last report:

```json
{
    "@context": "https://raw.githubusercontent.com/smart-data-models/dataModel.Device/master/context.jsonld",
    "id": "urn:ngsi-ld:Device:bgs-91",
    "type": "Device",
    "name": {
        "type": "Property",
        "value": "Water Temperature Logger 8OB101"
    },
    "description": {
        "type": "Property",
        "value": "Water Temperature Logger 8OB101"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -3.201234,
                51.482671
            ]
        }
    },
    "deviceCategory": {
        "type": "Property",
        "value": [
            "sensor"
        ]
    },
    "controlledProperty": {
        "type": "Property",
        "value": [
            "temperature"
        ]
    },
    "provider": {
        "type": "Property",
        "value": "British Geological Survey"
    },
    "dateFirstUsed": {
        "type": "Property",
        "value": "2015-06-19T00:00:00Z",
        "valueType": "DateTime"
    },
    "dateLastValueReported": {
        "type": "Property",
        "value": "2023-08-16T18:30:00Z",
        "valueType": "DateTime"
    }
}
```

`deviceCategory` and `controlledProperty` hold arrays as their values, since a Property's value can be any JSON value (ETSI GS CIM 009 v1.9.1, clause 4.5.2). The fibre-optic profile `DTS Profile TH0418`, `urn:ngsi-ld:Device:bgs-373`, comes out like the GasClam probe: a Device without `dateLastValueReported`, since its only readings would be in a MultiDatastream. BGS edits its Things as its stations change. While this example was written, 34 of them gained or lost their Locations between two runs an hour apart. The number of Devices with a `location` therefore moves from run to run, while the number of Devices stays the same.
