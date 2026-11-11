# Mapping roadworks with mixed and projected geometries

This example maps the roadworks of two German cities to one custom `RoadWork` model, and each city's geometry is awkward in its own way. Berlin publishes a roadwork as a marker point together with the lines of the road it closes, a combination NGSI-LD does not admit as a GeoProperty. Munich publishes its roadworks as surfaces through a WFS that answers in a projected coordinate system unless it is asked for WGS 84. The mapping keeps Berlin's whole geometry beside a GeoProperty that holds the marker, reads Munich's in WGS 84, and takes the time Munich's server answered from the collection around the features. [Example 31](../31-geojson-geometry-conversion/example.md) converted geometries that a GeoProperty could not take as they were. Here, one geometry cannot be converted without losing what it says.

## Get the data

The Verkehrsinformationszentrale Berlin (VIZ) publishes the roadworks, closures, and other disruptions of particular interest to traffic across the city, for the Senate Department for Mobility, Transport, Climate Protection and the Environment. The feed needs no key, is licensed under the Data licence Germany, attribution, version 2.0, and listed 267 roadworks when this was written:

```bash
curl --fail --location --output data/berlin.geojson "https://api.viz.berlin.de/daten/baustellen_sperrungen_viz.json"
```

Munich's mobility department (Mobilitätsreferat) publishes the current and planned roadworks and temporary no-parking zones of the coming four weeks through the city's GeoServer, under the same licence. The request asks the WFS for GeoJSON in EPSG:4326, and it listed 5,932 roadworks:

```bash
curl --fail --location --output data/munich.geojson "https://geoportal.muenchen.de/geoserver/mor_wfs/ows?service=WFS&version=1.1.0&request=GetFeature&typeName=mor_wfs:baustellen_opendata&outputFormat=application/json&srsName=EPSG:4326"
```

Both lists change every day, as works start and end.

## The record shape

A Berlin roadwork is a feature whose geometry is a `GeometryCollection`: the marker a map shows, then the road it closes. This one closes a bridge in Prenzlauer Berg:

```json
{
    "type": "Feature",
    "properties": {
        "id": "285/2026",
        "lms_id": "LMS-BR/r_LMS-BR/403521_LMS-BR/72",
        "tstore": "2026-04-14T15:45:17.545917Z",
        "objectState": "modified",
        "subtype": "Sperrung",
        "icon": "sperrung",
        "severity": "Vollsperrung",
        "direction": "Beidseitig",
        "street": "Dunckerstraße (Prenzlauer Berg)",
        "section": "in beiden Richtungen zwischen Ahlbecker Straße und Wichertstraße",
        "content": "Neubau Dunckerbrücke, Vollsperrung für den Kfz-, Rad- und Fußverkehr. Umleitungen sind ausgeschildert.",
        "validity": {
            "from": "2026-04-01T07:00",
            "to": "2028-06-14T17:00"
        }
    },
    "geometry": {
        "type": "GeometryCollection",
        "geometries": [
            {
                "type": "Point",
                "coordinates": [
                    13.423812030523553,
                    52.545907670453865
                ]
            },
            {
                "type": "LineString",
                "coordinates": [
                    [
                        13.423276840112525,
                        52.54530311985563
                    ],
                    [
                        13.424630641238764,
                        52.54681056775852
                    ]
                ]
            }
        ]
    }
}
```

The Berlin mapping reads `id`, `subtype`, `street`, `section`, `content`, `severity`, and `tstore`, and the geometry, and ignores the rest. 266 of the 267 roadworks have a collection like this one, a Point followed by one to five LineStrings. The other is stored as a lone Point.

A Munich roadwork is a surface, a `Polygon` or a `MultiPolygon`. The WFS answer carries five members of its own beside `features`, which GeoServer writes into every GeoJSON response it serves:

```json
{
    "type": "FeatureCollection",
    "features": [
        {
            "type": "Feature",
            "id": "baustellen_opendata.4",
            "geometry": {
                "type": "Polygon",
                "coordinates": [
                    [
                        [
                            11.56428057,
                            48.15101941
                        ],
                        [
                            11.56409667,
                            48.15107708
                        ],
                        [
                            11.56284142,
                            48.14913969
                        ],
                        [
                            11.56301679,
                            48.14908948
                        ],
                        [
                            11.56428057,
                            48.15101941
                        ]
                    ]
                ]
            },
            "geometry_name": "shape",
            "properties": {
                "strasse_hausnr": "Augustenstraße 53 - 95",
                "betroffene_bereiche": "Gehweg,Radweg,Baumgraben,Parkbucht,Fahrbahn",
                "beschreibung": "Bauabschnitt 2026",
                "beeintraechtigung": "Radweg verengt,Fahrbahn verengt,Einbahnregelung in Richtung Süden,Zeitweise Vollsperre",
                "art": "Baumaßnahme",
                "weitere_info": "muenchen.de/augustenstrasse",
                "kontakt_oeffentlich": null,
                "beginn_datum_kombiniert": "09.03.2026",
                "ende_datum_kombiniert": "31.12.2026",
                "fachliche_id": null
            }
        }
    ],
    "totalFeatures": 5932,
    "numberMatched": 5932,
    "numberReturned": 5932,
    "timeStamp": "2026-11-11T10:18:43.813Z",
    "crs": {
        "type": "name",
        "properties": {
            "name": "urn:ogc:def:crs:EPSG::4326"
        }
    }
}
```

The Munich mapping reads the feature's `id`, `art`, `strasse_hausnr`, `beschreibung`, `beeintraechtigung`, and geometry, and the collection's `timeStamp`, and ignores the rest.

## A GeometryCollection is not a GeoProperty

A GeoProperty's value is a GeoJSON geometry of any type except `GeometryCollection` (ETSI GS CIM 009 v1.9.1, clause 4.7.1). Reading Berlin's `{{ geometry }}` as the `location` GeoProperty therefore drops the attribute from 266 entities, and the warning suggests the one conversion that can fold a collection:

```text
! Attribute `location` dropped: a GeometryCollection is not an admissible GeoProperty value (ETSI GS CIM 009 v1.9.1 clause 4.7); declare `geometry: { convert: "flatten" }` to fold it into one geometry
```

`flatten` turns a collection of one dimension into the matching multi-geometry, so a collection of lines would become a `MultiLineString`. Berlin's collections mix a point with lines, and no single geometry type holds both, so `flatten` refuses them too:

```text
! Attribute `location` dropped: a GeometryCollection of mixed geometry types cannot be folded into one geometry
```

The marker is the right `location` for a map pin and for a `near` geo-query, and it is always the collection's first member, so the GeoProperty takes it:

```json5
location: {
    source: "{{ geometry.geometries | default(value=[geometry]) | first }}",
    type: "GeoProperty",
    transformation: "point",
},
```

`geometry.geometries` is the collection's member list and `first` takes its first entry. The lone Point has no `geometries`, so `default` stands in a list holding the geometry itself, and `first` takes that. The template is one expression, so its value keeps its type and reaches the `point` transformation as a GeoJSON object, which the transformation checks is a `Point`.

## Keeping the whole collection

The marker alone loses the closed road. A Property, unlike a GeoProperty, may hold any JSON value, so a Property with `transformation: "geometry"` keeps the collection as its GeoJSON object:

```json5
sourceGeometry: {
    source: "{{ geometry }}",
    type: "Property",
    transformation: "geometry",
},
```

The `geometry` transformation still reads every member as a geometry of its own: a member that is no valid geometry, or whose positions fall outside WGS 84, drops the attribute with a warning, and the attribute's `geometry` block applies its `winding` and `altitude` choices to each member. The lone Point is kept as the Point it is.

A broker stores this value as JSON and nothing more. It does not index it as a geometry, so geo-queries look at `location` alone. Keep the collection for consumers that draw or analyse the closure, and let the GeoProperty answer the queries.

## Asking a WFS for WGS 84

GeoJSON coordinates are WGS 84 longitude and latitude in decimal degrees (RFC 7946, section 4). A WFS answers in whatever coordinate system its `srsName` asks for, and in its own when there is none. Without `&srsName=EPSG:4326`, Munich's server answers in EPSG:25832, ETRS89 / UTM zone 32N, so a position reads `[688503.29000155, 5331930.82999633]` in metres. Cassiopeia checks every position against WGS 84 as it reads a geometry, and refuses such a run's every `location`:

```text
! Attribute `location` dropped: the position `[688503.29000155, 5331930.82999633]` at `coordinates[0][0][0]` has a longitude outside WGS 84; GeoJSON coordinates are WGS 84 longitude and latitude in decimal degrees (RFC 7946 section 4), so the source may be in a coordinate reference system it does not declare, have longitude and latitude swapped, or need another conversion or scale
```

The entities are still written, without `location`, and the run counts one `geometry-coordinates-out-of-range` warning for each, 5,932 of them. GeoServer names its coordinate system in a `crs` member, which the 2008 GeoJSON specification defined and RFC 7946 removed. Cassiopeia does not reproject from it. The fix belongs in the request, which is why the download asks for `srsName=EPSG:4326`.

The check sees numbers outside the range of degrees, not a wrong coordinate system as such. Longitude and latitude swapped in Germany, `[48.15, 11.56]`, are both legal degrees and pass. The check catches what can be caught without knowing where the data should be.

## Reading the collection's members

A feature's own members say what the roadwork is. GeoServer states when it answered once, in the collection's `timeStamp`, rather than on every feature. Cassiopeia gives every feature's record the members of its `FeatureCollection` other than `type` and `features` under `featureCollection`, so each Munich roadwork reads the time of the response it came in:

```json5
dateObserved: {
    source: "{{ featureCollection.timeStamp }}",
    transformation: "datetime",
},
```

Every Munich entity of one run gets the same `dateObserved`, the moment the list was true. Berlin has no such member and dates each record itself, by when the VIZ last stored it, which the Berlin mapping writes as `dateModified`. A Berlin feature's record has an empty `featureCollection`, because its collection carries nothing beyond its features.

The same members are open to any template: `{{ featureCollection.numberMatched }}` is the number of roadworks the server matched, and `{{ featureCollection.crs.properties.name }}` the coordinate system it answered in.

## Write the mapping

The manifest maps each city's file with its own mapping, and both produce `RoadWork` entities:

```json5
{
    version: "v1",
    inputs: [
        {
            source: "data/berlin.geojson",
            format: "geojson",
            mapping: "berlin.json5",
        },
        {
            source: "data/munich.geojson",
            format: "geojson",
            mapping: "munich.json5",
        },
    ],
}
```

The Berlin mapping, in [berlin.json5](berlin.json5), keys each roadwork by the VIZ's number, which carries its year after a slash, as in `285/2026`. A slash in an entity's ID would split it in a broker's URL path, so `replace` turns it into a hyphen:

```json5
{
    version: "v4",
    dataModel: "RoadWork",
    identity: {
        entityName: "berlin-{{ properties.id | replace(from='/', to='-') }}",
    },
    attributes: {
        category: {
            source: "{{ properties.subtype }}",
        },
        street: {
            source: "{{ properties.street }}",
        },
        section: {
            source: "{{ properties.section }}",
        },
        description: {
            source: "{{ properties.content }}",
        },
        restriction: {
            source: "{{ properties.severity }}",
        },
        dateModified: {
            source: "{{ properties.tstore }}",
            transformation: "datetime",
        },
        location: {
            source: "{{ geometry.geometries | default(value=[geometry]) | first }}",
            type: "GeoProperty",
            transformation: "point",
        },
        sourceGeometry: {
            source: "{{ geometry }}",
            type: "Property",
            transformation: "geometry",
        },
    },
}
```

The Munich mapping, in [munich.json5](munich.json5), keys each roadwork by the number GeoServer puts after the layer's name in the feature ID, as in `baustellen_opendata.4`:

```json5
{
    version: "v4",
    dataModel: "RoadWork",
    identity: {
        entityName: "munich-{{ id | split(pat='.') | last }}",
    },
    attributes: {
        category: {
            source: "{{ properties.art }}",
        },
        street: {
            source: "{{ properties.strasse_hausnr }}",
        },
        description: {
            source: "{{ properties.beschreibung }}",
        },
        restriction: {
            source: "{{ properties.beeintraechtigung }}",
        },
        location: {
            source: "{{ geometry }}",
            type: "GeoProperty",
        },
        dateObserved: {
            source: "{{ featureCollection.timeStamp }}",
            transformation: "datetime",
        },
    },
}
```

A Munich geometry is already a `Polygon` or a `MultiPolygon`, so the default `geometry` transformation takes it as it is.

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

From the repository root, the runner downloads both datasets, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 42
cargo run -- run 42 --runtime docker
```

The custom model needs no context or schema. The run writes `out/RoadWork.json`, one entity per roadwork from both cities, 6,199 when this was written.

## Read the result

The Dunckerbrücke closure has its marker as `location` and the whole collection, marker and closed road, as `sourceGeometry`:

```json
{
    "id": "urn:ngsi-ld:RoadWork:berlin-285-2026",
    "type": "RoadWork",
    "category": {
        "type": "Property",
        "value": "Sperrung"
    },
    "street": {
        "type": "Property",
        "value": "Dunckerstraße (Prenzlauer Berg)"
    },
    "section": {
        "type": "Property",
        "value": "in beiden Richtungen zwischen Ahlbecker Straße und Wichertstraße"
    },
    "description": {
        "type": "Property",
        "value": "Neubau Dunckerbrücke, Vollsperrung für den Kfz-, Rad- und Fußverkehr. Umleitungen sind ausgeschildert."
    },
    "restriction": {
        "type": "Property",
        "value": "Vollsperrung"
    },
    "dateModified": {
        "type": "Property",
        "value": "2026-04-14T15:45:17.545917Z",
        "valueType": "DateTime"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                13.423812030523553,
                52.545907670453865
            ]
        }
    },
    "sourceGeometry": {
        "type": "Property",
        "value": {
            "type": "GeometryCollection",
            "geometries": [
                {
                    "type": "Point",
                    "coordinates": [
                        13.423812030523553,
                        52.545907670453865
                    ]
                },
                {
                    "type": "LineString",
                    "coordinates": [
                        [
                            13.423276840112525,
                            52.54530311985563
                        ],
                        [
                            13.424630641238764,
                            52.54681056775852
                        ]
                    ]
                }
            ]
        }
    }
}
```

`sourceGeometry` is a `Property`, not a `GeoProperty`, so its value is an ordinary JSON object that happens to be GeoJSON. `dateModified` keeps all six fractional digits the source wrote, the most an NGSI-LD `DateTime` admits, and its `valueType` names the datatype.

The Augustenstraße works in Munich have their surface as `location`, in degrees, and the time of the response as `dateObserved`:

```json
{
    "id": "urn:ngsi-ld:RoadWork:munich-4",
    "type": "RoadWork",
    "category": {
        "type": "Property",
        "value": "Baumaßnahme"
    },
    "street": {
        "type": "Property",
        "value": "Augustenstraße 53 - 95"
    },
    "description": {
        "type": "Property",
        "value": "Bauabschnitt 2026"
    },
    "restriction": {
        "type": "Property",
        "value": "Radweg verengt,Fahrbahn verengt,Einbahnregelung in Richtung Süden,Zeitweise Vollsperre"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Polygon",
            "coordinates": [
                [
                    [
                        11.56428057,
                        48.15101941
                    ],
                    [
                        11.56409667,
                        48.15107708
                    ],
                    [
                        11.56284142,
                        48.14913969
                    ],
                    [
                        11.56301679,
                        48.14908948
                    ],
                    [
                        11.56428057,
                        48.15101941
                    ]
                ]
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-11T10:18:43.813Z",
        "valueType": "DateTime"
    }
}
```

The `dateObserved` is the `timeStamp` of the collection quoted above, carried onto this one feature's entity.
