# Building a CSV relationship graph across five sources

The [airport and airline example](../09-csv-manifest/example.md) becomes a connected graph here. Five OpenFlights files provide countries, aircraft models, airports, airlines, and routes. Routes point to an airline, two airports, and an aircraft model; airports and airlines point to countries. Four types use published Smart Data Models, while `Country` is a custom relationship target. Two of the lists repeat some entities, so a short awk step collapses them to one row per entity first. One manifest then builds the graph.

## Get the data

All five files come from [OpenFlights](https://github.com/jpatokal/openflights), under the Open Database License (ODbL):

```bash
curl --fail --location --output data/countries.dat https://raw.githubusercontent.com/jpatokal/openflights/master/data/countries.dat
curl --fail --location --output data/planes.dat https://raw.githubusercontent.com/jpatokal/openflights/master/data/planes.dat
curl --fail --location --output data/airports.dat https://raw.githubusercontent.com/jpatokal/openflights/master/data/airports.dat
curl --fail --location --output data/airlines.dat https://raw.githubusercontent.com/jpatokal/openflights/master/data/airlines.dat
curl --fail --location --output data/routes.dat https://raw.githubusercontent.com/jpatokal/openflights/master/data/routes.dat
```

## Get the schemas

Four of the models are published Smart Data Models. Download the catalog once:

```bash
cassiopeia sdm download
```

## One row per entity

Two of the files list some entities more than once. Routes name their aircraft by IATA code, so the aircraft mapping keys each `AircraftModel` on that code, but one IATA code can cover several rows of `planes.dat`: `CN1` stands for four Cessna types, and `E7W` for both wing variants of the Embraer 175. Airports name their country as text, so the country mapping keys each `Country` on its name, but `countries.dat` has one row per DAFIF code rather than per country: Palestine appears once as `GZ` and once as `WE`.

Rows that resolve to the same ID merge into one entity, as the [ID collision example](../02-json-id-collision/example.md) shows. Where their values disagree, only one row's values survive: the `E7W` entity would describe either the long-wing or the short-wing model, and Palestine would keep one DAFIF code and lose the other.

Two small awk scripts therefore collapse both lists to one row per entity ID before mapping. They share a CSV reader, [openflights-csv.awk](openflights-csv.awk):

```bash
awk -f openflights-csv.awk -f collapse-countries.awk data/countries.dat > data/countries.csv
awk -f openflights-csv.awk -f collapse-planes.awk data/planes.dat > data/aircraft-models.csv
```

[collapse-planes.awk](collapse-planes.awk) merges the rows that share an IATA code, and passes a row without one through unchanged. The merged name lists every model the code covers, separated by semicolons because the names themselves already use slashes. The script keeps the ICAO code only when the merged rows agree on one. `E75L` and `E75S` cannot both be the ICAO code of `E7W`, so these two rows:

```text
"Embraer 175 (long wing)","E7W","E75L"
"Embraer 175 (short wing)","E7W","E75S"
```

become one row with no ICAO code:

```text
"Embraer 175 (long wing); Embraer 175 (short wing)","E7W",\N
```

[collapse-countries.awk](collapse-countries.awk) merges the rows that share a name and puts all of their DAFIF codes in one column, separated by spaces:

```text
"Palestine","PS","GZ WE"
"United Kingdom","GB","UK"
```

The rows of one country should agree on its ISO code. If a later version of the list gives a country two, the script stops with an error instead of silently keeping one of them.

The 246 aircraft rows become 232 models and the 261 country rows become 259 countries. The other three files need no preparation: each of their rows already has an ID of its own.

## The graph

The source rows already contain the IDs or names needed for these links:

```text
Country  ←  Airport  ←  Route  →  Airline  →  Country
                ↑         │
                └─────────┤
                          ↓
                    AircraftModel
```

A route names its airline, departure and arrival airports, and equipment. An airport and an airline each name their country.

## Two ways to key a relationship

As in the [previous relationship example](../10-json-relationships/example.md), each relationship object URN combines a source value with the target model. This graph uses two kinds of source value.

Most links use an **id**. A route row carries OpenFlights IDs for its airline and airports, so those relationships read the value directly:

```json5
belongsToAirline: {
    source: "{{ this[1] }}",
    type: "Relationship",
    target: {
        entity: "Airline",
    },
},
departsFromAirport: {
    source: "{{ this[3] }}",
    type: "Relationship",
    target: {
        entity: "Airport",
    },
},
```

The airport-to-country link uses a **name** because the airport file stores the country as text rather than as an ID. The country mapping uses that name as its identity, so both sides produce the same URN:

```json5
// in country.json5
identity: {
    entityName: "{{ this[0] }}",
},
// in airport.json5
inCountry: {
    source: "{{ this[3] }}",
    type: "Relationship",
    target: {
        entity: "Country",
    },
},
```

An airport in the United Kingdom points to `urn:ngsi-ld:Country:UnitedKingdom`, the ID produced for the United Kingdom row. A few names differ between the two files; those links are valid URNs, but no country entity matches them. The aircraft link has the same gap: routes use some IATA codes that the aircraft list does not have, such as `73H` and `CRJ`, so those routes point to an `AircraftModel` that the run never writes.

## The route mapping

A route has no source ID, so its identity combines the airline and its two endpoints. Each triple is distinct, making each route its own `Flight`. The equipment column can list several aircraft codes, but the Flight model defines `hasAircraftModel` as one relationship, so this mapping keeps the first code. Text values are split; an all-numeric code is already read as a number and is used as-is:

```json5
{
    version: "v4",
    dataModel: "dataModel.Aeronautics/Flight",
    identity: {
        entityName: "{{ this[0] }}-{{ this[2] }}-{{ this[4] }}",
    },
    attributes: {
        belongsToAirline: {
            source: "{{ this[1] }}",
            type: "Relationship",
            target: {
                entity: "Airline",
            },
        },
        departsFromAirport: {
            source: "{{ this[3] }}",
            type: "Relationship",
            target: {
                entity: "Airport",
            },
        },
        arrivesToAirport: {
            source: "{{ this[5] }}",
            type: "Relationship",
            target: {
                entity: "Airport",
            },
        },
        hasAircraftModel: {
            source: "{% if this[8] is string %}{{ this[8] | split(pat=' ') | first }}{% else %}{{ this[8] }}{% endif %}",
            type: "Relationship",
            target: {
                entity: "AircraftModel",
            },
        },
    },
}
```

The Flight model defines all four relationships, so the resulting entity validates against its schema.

## The country, without a schema

The country list is the only source without a published model, so `country.json5` uses a plain `Country` type. The other four types have Smart Data Model schemas. In `fail-when-schema` mode, Cassiopeia validates those four types and writes `Country` without a schema check.

A country can have several DAFIF codes, so `dafifCode` is always a list, even when it holds one code. That way, a consumer never has to check whether it got a string or an array. `split` turns the space-separated column into a list, and the `array` transformation keeps it as one:

```json5
dafifCode: {
    source: "{% if this[2] %}{{ this[2] | split(pat=' ') }}{% endif %}",
    type: "Property",
    transformation: "array",
},
```

The order of the codes carries no meaning, so this is a plain `Property` with an array value rather than a `ListProperty`. Bonaire, Saint Eustatius and Saba has no DAFIF code at all. The guard skips `split` for it, and its entity has no `dafifCode`.

## The manifest

```json5
{
    version: "v1",
    inputs: [
        {
            source: "data/countries.csv",
            mapping: "country.json5",
            format: "csv",
        },
        {
            source: "data/aircraft-models.csv",
            mapping: "plane.json5",
            format: "csv",
        },
        {
            source: "data/airports.dat",
            mapping: "airport.json5",
            format: "csv",
        },
        {
            source: "data/airlines.dat",
            mapping: "airline.json5",
            format: "csv",
        },
        {
            source: "data/routes.dat",
            mapping: "route.json5",
            format: "csv",
        },
    ],
    output: {
        target: "file",
        directory: "out",
        context: "none",
        validation: {
            mode: "fail-when-schema",
        },
    },
}
```

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
    ghcr.io/vela-tools/cassiopeia:v1.1.1 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 11
cargo run -- run 11 --runtime docker
```

The run writes `Country.json`, `AircraftModel.json`, `Airport.json`, `Airline.json`, and `Flight.json`. The airport and airline counts match the previous example, and all 67,663 routes become `Flight` entities.

## Read the result

A `Flight` is a junction entity: one ID and four relationships to the other types.

```json
{
    "id": "urn:ngsi-ld:Flight:LH-EDI-FRA",
    "type": "Flight",
    "belongsToAirline": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:Airline:3320",
        "objectType": "Airline"
    },
    "departsFromAirport": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:Airport:535",
        "objectType": "Airport"
    },
    "arrivesToAirport": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:Airport:340",
        "objectType": "Airport"
    },
    "hasAircraftModel": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:AircraftModel:321",
        "objectType": "AircraftModel"
    }
}
```

London Heathrow points at its country, which is a real entity in the same run:

```json
{
    "id": "urn:ngsi-ld:Airport:507",
    "type": "Airport",
    "name": {
        "type": "Property",
        "value": "London Heathrow Airport"
    },
    "codeIATA": {
        "type": "Property",
        "value": "LHR"
    },
    "inCountry": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:Country:UnitedKingdom",
        "objectType": "Country"
    }
}
```

```json
{
    "id": "urn:ngsi-ld:Country:UnitedKingdom",
    "type": "Country",
    "name": {
        "type": "Property",
        "value": "United Kingdom"
    },
    "isoCode": {
        "type": "Property",
        "value": "GB"
    },
    "dafifCode": {
        "type": "Property",
        "value": [
            "UK"
        ]
    }
}
```

The Embraer 175 is one `AircraftModel` for both wing variants that `E7W` covers. Its name lists both, and it has no `codeICAO` because the two variants have different ICAO codes:

```json
{
    "id": "urn:ngsi-ld:AircraftModel:E7W",
    "type": "AircraftModel",
    "name": {
        "type": "Property",
        "value": "Embraer 175 (long wing); Embraer 175 (short wing)"
    },
    "codeIATA": {
        "type": "Property",
        "value": "E7W"
    }
}
```

This mapping stays within the Flight Smart Data Model by keeping only a route's first aircraft, because `hasAircraftModel` is a single relationship. Routes often list several aircraft. The [next example](../12-csv-list-relationship/example.md) keeps them all with a newer NGSI-LD type that the published schema does not describe.

## Carrying every aircraft, on-spec

Reducing the equipment to its primary aircraft is a modelling choice, not the only one. To carry _all_ of a route's aircraft while still validating against the Flight model, you would keep `hasAircraftModel` as a `Relationship` but give it several instances, one per aircraft, each an object distinguished by its own `datasetId` (ETSI GS CIM 009 v1.9.1 clause 4.5.5). A _multi-attribute_ is one attribute name holding several instances, serialized as a JSON array; at most one instance may omit its `datasetId`, so linking to more than one aircraft means every additional instance names its own. Each link stays a plain `Relationship` the schema recognises, which is the difference from the [`ListRelationship`](../12-csv-list-relationship/example.md) the next example reaches for: that gathers the aircraft into one ordered `objectList` the Flight model does not describe, whereas the multi-attribute keeps the shape the model already expects. The [datasetId example](../21-json-dataset-id/example.md) works the same multi-instance mechanism on a Property.
