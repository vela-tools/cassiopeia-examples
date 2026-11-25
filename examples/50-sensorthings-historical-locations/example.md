# Reading where a SensorThings Thing has been

A record's `location` is where the Thing is when the read runs. A SensorThings server also keeps where a Thing was, and when: each time a Thing's Location changes, the server records a HistoricalLocation (OGC 18-088, section 8.2.3). This example reads three months of a Hamburg bicycle counting station that received a new position in that time, with `historicalLocations: true`. It writes the station as one `TrafficFlowObserved` that holds its daily counts and its places as series.

## Get the data

The Free and Hanseatic City of Hamburg publishes the sensors of its [Urban Data Platform](https://www.hamburg.de/) on a FROST-Server at `https://iot.hamburg.de/v1.1`: traffic counters, bicycle counting stations, charging points and more. The city publishes the platform's data under the [Datenlizenz Deutschland Namensnennung 2.0](https://www.govdata.de/dl-de/by-2-0); the server itself states no licence.

The station is Thing 5564, `Verkehrszählstelle 0295970 (veraltet)`, an infrared counter of the city's bicycle counting network, now marked obsolete and replaced by a new Thing. Its Datastream 11798 holds one count per day. This request lists its HistoricalLocations, each with the Location it points to:

```bash
curl --globoff "https://iot.hamburg.de/v1.1/Things(5564)/HistoricalLocations?\$expand=Locations(\$select=location)&\$select=time"
```

Two of them, on 2020-10-02 and on 2020-11-16, put the station at points about 30 metres apart. The server does not say why. A counter's recorded position is often corrected after installation without the counter itself moving. The example makes no claim either way and writes what the server records.

## Get the schema

```bash
cassiopeia sdm download
```

## Reading HistoricalLocations

```json5
sensorthings: {
    datastreams: "id eq 11798",
    read: { range: { from: "2020-09-01T00:00:00Z", to: "2020-12-01T00:00:00Z" } },
    historicalLocations: true,
},
```

A range read first reads the Observations of each data stream, as [example 44](../44-sensorthings-range-read/example.md) does. With `historicalLocations: true` it then requests the HistoricalLocations of each Thing those data streams belong to, within the same range, each with its Locations:

```text
{root}/Things(5564)/HistoricalLocations?$filter=time%20ge%202020-09-01T00:00:00Z%20and%20time%20lt%202020-12-01T00:00:00Z&$expand=Locations&$orderby=time%20asc,id%20asc&$top=1000
```

A latest read refuses the setting, because its record's `location` already holds the Thing's last known place. An incremental read, as in [example 46](../46-sensorthings-incremental-broker/example.md), keeps a cursor position per Thing for its HistoricalLocations, beside those of its data streams.

## The record shape

The read gives 90 records: 88 from the daily counts, one per Observation, and two from the HistoricalLocations, which come last. A HistoricalLocation's record holds the Thing's `server`, `thing`, `location` and `locations`, a `historicalLocation`, and no `readings`:

```json
{
    "server": {
        "url": "https://iot.hamburg.de/v1.1",
        "host": "iot.hamburg.de",
        "version": "v1.1"
    },
    "thing": {
        "id": 5564,
        "name": "Verkehrszählstelle 0295970 (veraltet)",
        "description": "Verkehrszählstelle zur Zählung der vom Infrarotdetektor erfassten Mobilitätswerkzeuge",
        "properties": {
            "assetID": "0295970",
            "infoLastUpdate": "2021-09-16T07:59:31.678Z",
            "keywords": ["Infrarotdetektor", "Verkehrsmenge", "automatisierte Verkehrsmengenerfassung", "Hamburger Radzählnetz", "Hamburg", "aVME", "HaRaZäN", "Zählstelle"],
            "language": "de",
            "ownerThing": "Freie und Hansestadt Hamburg",
            "richtung": "Querschnitt"
        }
    },
    "location": {
        "type": "Point",
        "coordinates": [9.999098, 53.579928]
    },
    "locations": [
        {
            "id": 26517652,
            "name": "Verkehrszählstelle 0295970",
            "description": "Mittelpunkt des Streckenverlaufes bezüglich Zählstelle 0295 auf dem Richtungsarm Südwest für den Zählquerschnitt",
            "encodingType": "application/vnd.geo+json",
            "location": {
                "type": "Point",
                "coordinates": [9.999098, 53.579928]
            }
        }
    ],
    "historicalLocation": {
        "id": 26516625,
        "time": "2020-10-02T11:00:29.029689Z",
        "location": {
            "type": "Point",
            "coordinates": [9.999377, 53.580126]
        }
    }
}
```

`historicalLocation.time` is the HistoricalLocation's `time` in UTC. `historicalLocation.location` is the geometry of its GeoJSON Location, read the same way as the record's `location`. This server writes each Location as a GeoJSON `Feature`, and Cassiopeia takes its `geometry`. The record's own `location` is still the station's current place, the same on every record.

A count's record holds one reading, keyed by its ObservedProperty's name, `Anzahl Fahrräder je Zählstelle (veraltet)`, "number of bicycles per counting station". Its `phenomenonTime` is the day the count covers in Hamburg's time, such as `2020-09-01T22:00:00Z/2020-09-02T21:59:59Z`, and its `time` is the end of that day.

## A track as location instances

```json5
location: {
    type: "GeoProperty",
    source: "{{ historicalLocation.location }}",
    properties: {
        observedAt: {
            source: "{{ historicalLocation.time }}",
            transformation: "datetime",
        },
    },
},
```

On a HistoricalLocation's record, this gives the place and the time it was recorded. On a count's record there is no `historicalLocation`, so the plain references resolve to nothing and `location` is left out of that record. Series output then gathers the instances of each attribute across the entity's records, so `location` holds one instance per HistoricalLocation, oldest first: the station's track.

The counts work the other way round. The reading's key has spaces, parentheses and umlauts, so the template reads it with brackets. A HistoricalLocation's record has no `readings`, and the guard leaves `intensity` out of it. Without the guard, the template would print a field the record does not have, which fails, whereas a plain reference would resolve to nothing.

```json5
intensity: {
    source: "{% if readings %}{{ readings['Anzahl Fahrräder je Zählstelle (veraltet)'].result }}{% endif %}",
    transformation: "integer",
    properties: {
        observedAt: {
            source: "{% if readings %}{{ readings['Anzahl Fahrräder je Zählstelle (veraltet)'].time }}{% endif %}",
            transformation: "datetime",
        },
    },
},
```

`dateObserved`, which `TrafficFlowObserved` requires, holds the interval each count covers, which the model allows. It carries the same `observedAt`, so it forms a series beside `intensity`, one instance per day. `vehicleType` is `bicycle`. The identity uses the server's id, `hamburg-{{ thing.id }}`, because the asset number 0295970 also names the Thing that replaced this station, and that Thing has data of its own.

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
cargo run -- run 50
cargo run -- run 50 --runtime docker
```

The read takes about two seconds and writes one entity to `out/TrafficFlowObserved.jsonld`.

## Read the result

The entity holds 88 counts and the two places. Its `intensity` and `dateObserved` arrays are cut to their first two instances here:

```json
{
    "@context": "https://raw.githubusercontent.com/smart-data-models/dataModel.Transportation/master/context.jsonld",
    "id": "urn:ngsi-ld:TrafficFlowObserved:hamburg-5564",
    "type": "TrafficFlowObserved",
    "name": {
        "type": "Property",
        "value": "Verkehrszählstelle 0295970 (veraltet)"
    },
    "description": {
        "type": "Property",
        "value": "Verkehrszählstelle zur Zählung der vom Infrarotdetektor erfassten Mobilitätswerkzeuge"
    },
    "vehicleType": {
        "type": "Property",
        "value": "bicycle"
    },
    "intensity": [
        {
            "type": "Property",
            "value": 11821,
            "observedAt": "2020-09-02T21:59:59Z"
        },
        {
            "type": "Property",
            "value": 8303,
            "observedAt": "2020-09-03T21:59:59Z"
        }
    ],
    "dateObserved": [
        {
            "type": "Property",
            "value": "2020-09-01T22:00:00Z/2020-09-02T21:59:59Z",
            "observedAt": "2020-09-02T21:59:59Z"
        },
        {
            "type": "Property",
            "value": "2020-09-02T22:00:00Z/2020-09-03T21:59:59Z",
            "observedAt": "2020-09-03T21:59:59Z"
        }
    ],
    "location": [
        {
            "type": "GeoProperty",
            "value": {
                "type": "Point",
                "coordinates": [
                    9.999377,
                    53.580126
                ]
            },
            "observedAt": "2020-10-02T11:00:29.029689Z"
        },
        {
            "type": "GeoProperty",
            "value": {
                "type": "Point",
                "coordinates": [
                    9.999098,
                    53.579928
                ]
            },
            "observedAt": "2020-11-16T13:05:45.387678Z"
        }
    ]
}
```

`location` is a GeoProperty with two instances, each with its `observedAt`. NGSI-LD writes any Property whose value changed over time this way, geospatial ones included (ETSI GS CIM 009 v1.9.1, clauses 4.7 and 4.5.5). A broker's temporal API stores it the same way, and a temporal query on the entity's `location` returns the track. The two points are the server's record of where the station was. The server records nothing about the station's place before the first point or between the two, so the entity holds nothing for those times either.

A HistoricalLocation records where the Thing was. Where a reading was taken is its FeatureOfInterest, which `featuresOfInterest: true` reads, as in [example 51](../51-sensorthings-taiwan-end-to-end/example.md). For a station the two are the same place. For a sensor on a bus, each reading's FeatureOfInterest places that reading, and the HistoricalLocations give the route between readings.
