# Placing entities in NGSI-LD scopes

An NGSI-LD scope puts an entity into a hierarchy of the producer's own design, by location or by organization, and a consumer can then ask a broker for the entities in one part of it. This example maps NOAA's water-level stations to `WaterLevelStation` entities and gives each station one scope for every observing network it belongs to, plus one for the Great Lakes. A station belongs to a varying number of networks, so the mapping reads them as a list. The stations sit one level down in the API's answer, which the manifest reads as a nested collection, as [example 32](../32-json-nested-collections/example.md) introduced.

## Get the data

NOAA's Center for Operational Oceanographic Products and Services (CO-OPS) runs the tide and water-level gauges along the US coasts and on the Great Lakes. Its [Metadata API](https://api.tidesandcurrents.noaa.gov/mdapi/prod/) needs no key. NOAA puts the information on its servers in the public domain unless it is annotated otherwise, and asks for attribution when the material is redistributed. The list of water-level stations holds about 300 of them:

```bash
curl --fail --location --output data/stations.json "https://api.tidesandcurrents.noaa.gov/mdapi/prod/webapi/stations.json?type=waterlevels"
```

The list is live: NOAA opens new stations and retires old ones.

## The record shape

The answer is an object holding a `count` and a `stations` array. This is one station, the gauge at De Tour Village on Lake Huron, with its thirteen link members left out:

```json
{
    "count": 303,
    "units": null,
    "self": null,
    "stations": [
        {
            "tidal": false,
            "greatlakes": true,
            "shefcode": "DTLM4",
            "state": "MI",
            "timezone": "EST",
            "timezonecorr": -5,
            "observedst": true,
            "stormsurge": false,
            "forecast": true,
            "outlook": false,
            "HTFhistorical": false,
            "HTFmonthly": false,
            "nonNavigational": false,
            "inundationdb": true,
            "id": "9075099",
            "name": "De Tour Village",
            "lat": 45.9925,
            "lng": -83.8982,
            "affiliations": "NWLON,NWLORTS",
            "portscode": "sl",
            "self": "https://api.tidesandcurrents.noaa.gov/mdapi/prod/webapi/stations/9075099.json",
            "expand": "details,sensors,floodlevels,datums,harcon,tidepredoffsets,ofsmapoffsets,products,disclaimers,notices",
            "tideType": "Great Lakes"
        }
    ]
}
```

Each member left out, such as `details` or `sensors`, is an object holding one `self` URL that leads to more metadata. The mapping reads `id`, `name`, `lat`, `lng`, `tidal`, `affiliations`, and `greatlakes`, and ignores the rest.

`affiliations` names the observing networks the station belongs to, separated by commas. The whole list uses nine such codes, among them `NWLON` for the National Water Level Observation Network, `PORTS` for the Physical Oceanographic Real-Time System, and `TCOON` for the Texas Coastal Ocean Observation Network. Most stations belong to one network, and 23 belong to between two and four. `greatlakes` is a JSON boolean, true for the 52 gauges on the Great Lakes.

## What a scope is

A scope is one or more levels separated by slashes, such as `/Madrid/Gardens/ParqueNorte` in the specification's own example. A level starts with a Unicode letter and continues with letters, numbers, or underscores (ETSI GS CIM 009 v1.9.1, clause 4.18). An entity can be in several scopes at once. A query's `scopeQ` selects the entities in one scope, in any of several, in all of several, or anywhere in a subtree.

A scope is not part of an entity's identity. NGSI-LD identifies an entity by its ID alone, and an entity's scopes can change after it is created, so the mapping declares `scope` at its top level, beside `identity`, not inside it.

## One scope per network

The networks are the scopes this example wants, but they arrive in one text field. Read as it is, `/{{ affiliations }}` works for a station in one network and fails for every other one, because a scope cannot hold a comma:

```text
! The resolved scope '/PORTS,TCOON' cannot be the entity's scope
  ╰─ '/PORTS,TCOON' is not a legal NGSI-LD scope: a scope is one or more '/'-separated levels, each starting with a Unicode letter and continuing with Unicode letters, numbers or underscores (ETSI GS CIM 009 v1.9.1 clause 4.18)
```

That run skips 23 stations, one for each station in more than one network, and counts each under `expander-urn-ungeneratable`. The rest of the run carries on.

A template that reads a list whole gives one scope per item, so the mapping turns the text into a list first:

```json5
"{{ ('/' ~ affiliations) | replace(from=',', to=',/') | split(pat=',') }}",
```

Read it from the inside out: `'/' ~ affiliations` puts a slash before the first name, `replace` puts one after every comma, and `split` cuts the text at the commas. `NWLON,NWLORTS` becomes the list `["/NWLON", "/NWLORTS"]`, and each item is checked as a scope on its own. The template is a single `{{ }}` expression, so its value keeps its type and stays a list. An item that is not a legal scope would skip the record with a warning that quotes that item alone, an empty item would give no scope, and so would an empty list.

The leading slash is optional in a scope but not in a scope query, which always starts with one (clause 4.19). Writing every scope with its slash keeps the stored scopes spelled the way the queries that ask for them are.

## A boolean is not a scope

The Great Lakes gauges are a natural group for a consumer to ask for. The tempting way to write that scope is `/GreatLakes/{{ greatlakes }}`, which would print the boolean as `true` or `false`: a legal level, but not a scope the data meant. Cassiopeia refuses it, and the run skips every record:

```text
! The scope template `/GreatLakes/{{ greatlakes }}` reads `greatlakes`, which holds a boolean, not a scope
```

The same holds for `{{ greatlakes }}` on its own, and for a number or an object read as the whole scope. A number printed into a level, as in `/Floor{{ floor }}`, is kept. Here the boolean decides whether the scope is there at all, so a condition says exactly that:

```json5
"{% if greatlakes %}/GreatLakes{% endif %}",
```

A coastal station renders this to nothing, which gives no scope. A boolean that describes the station belongs in a Property, where it keeps its type, as `tidal` does in this mapping.

## A scope cannot be the NGSI-LD Null

The scope grammar admits one more value, the NGSI-LD Null `urn:ngsi-ld:null`, but only to mark a scope that has been deleted. Cassiopeia writes entities and never deletes a scope, so a mapping whose `scope` is that literal does not load:

```text
× The scope template in the mapping document at 'station.json5' can never be an entity's scope
  ╰─ 'urn:ngsi-ld:null' cannot be the scope of an entity Cassiopeia writes: the NGSI-LD Null only marks a deleted scope (ETSI GS CIM 009 v1.9.1 clause 4.18)
```

A source field that holds the NGSI-LD Null is refused the same way, one record at a time. To take a scope away from an entity a broker already holds, update that entity at the broker.

## Write the mapping

The manifest lists the `stations` collection, the same way example 32 listed its nested arrays:

```json5
{
    version: "v1",
    inputs: [
        {
            source: "data/stations.json",
            format: "json",
            mappings: [
                {
                    collection: "stations",
                    mapping: "station.json5",
                },
            ],
        },
    ],
}
```

The complete mapping is in [station.json5](station.json5):

```json5
{
    version: "v4",
    dataModel: "WaterLevelStation",
    identity: {
        entityName: "{{ id }}",
    },
    scope: [
        "{{ ('/' ~ affiliations) | replace(from=',', to=',/') | split(pat=',') }}",
        "{% if greatlakes %}/GreatLakes{% endif %}",
    ],
    attributes: {
        name: {
            source: "{{ name }}",
            transformation: "string",
        },
        tidal: {
            source: "{{ tidal }}",
            transformation: "boolean",
        },
        location: {
            source: [
                "{{ lng }}",
                "{{ lat }}",
            ],
            type: "GeoProperty",
            transformation: "point",
        },
    },
}
```

The scopes of both entries are joined, duplicates are dropped, and the result is written sorted by Unicode code point, so the output is the same on every run. An entity with one scope gets a string and an entity with several gets an array.

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
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.1.1 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 41
cargo run -- run 41 --runtime docker
```

The custom model needs no context or schema. The run writes `out/WaterLevelStation.json`, one entity per station, 303 when this was written.

## Read the result

De Tour Village belongs to two networks and lies on Lake Huron, so it has three scopes, sorted:

```json
{
    "id": "urn:ngsi-ld:WaterLevelStation:9075099",
    "type": "WaterLevelStation",
    "scope": [
        "/GreatLakes",
        "/NWLON",
        "/NWLORTS"
    ],
    "name": {
        "type": "Property",
        "value": "De Tour Village"
    },
    "tidal": {
        "type": "Property",
        "value": false
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -83.8982,
                45.9925
            ]
        }
    }
}
```

`scope` is written as a plain value beside `id` and `type`, not as an attribute object with a `type` and a `value`: NGSI-LD makes it a special Property that the normalized representation does not reify. Port Arthur, on the Texas coast, is in two networks and not on the Great Lakes:

```json
{
    "id": "urn:ngsi-ld:WaterLevelStation:8770475",
    "type": "WaterLevelStation",
    "scope": [
        "/PORTS",
        "/TCOON"
    ],
    "name": {
        "type": "Property",
        "value": "Port Arthur"
    },
    "tidal": {
        "type": "Property",
        "value": true
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -93.93,
                29.8667
            ]
        }
    }
}
```

231 of the 303 stations have a single scope, which is written as a string, such as Providence's `"scope": "/NWLORTS"`.

Delivered to a broker, the scopes are what a consumer filters on. A `scopeQ` lists scopes with `,` for or, and joins them with `;` inside parentheses for and. Against the 303 stations above:

```text
scopeQ=/GreatLakes               52 stations
scopeQ=/PORTS,/TCOON             86 stations in either network
scopeQ=(/GreatLakes;/NWLORTS)     7 stations on the Great Lakes in NWLORTS
```
