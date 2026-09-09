# Mapping KMZ folder namespacing into entities

A KMZ of Yosemite National Park's points of interest becomes `ParkPointOfInterest` entities: waterfalls, trailheads, campgrounds, shuttle stops, and the restrooms that outnumber all of them. The [previous example](../15-shapefile-enum-decoding/example.md) read a zipped shapefile; this one reads the other zipped geographic format, and shows what a KML `<Folder>` does to the shape of a record. The [next example](../17-kml-folder-collections/example.md) uses several folders at once.

## Get the data

The dataset is [Yosemite National Park - Points Of Interest](https://hub.arcgis.com/datasets/nps::yosemite-national-park-points-of-interest-open-data), published by the National Park Service. It holds 553 placemarks covering natural features and visitor infrastructure, and every record is marked `Unrestricted`. As a work of the US federal government it carries no copyright; the licence text on the item is a liability disclaimer rather than a restriction on reuse.

The park service publishes it as KML. Download it and package it as the `.kmz` this example reads:

```bash
mkdir -p data
curl --fail --location \
    --output data/yosemite-poi.kml \
    "https://opendata.arcgis.com/datasets/nps::yosemite-national-park-points-of-interest-open-data.kml"
cd data && rm -f yosemite-poi.kmz && zip -q yosemite-poi.kmz yosemite-poi.kml && cd ..
```

A KMZ is a zip archive whose payload is a KML file, so packaging one is exactly that: zip the KML. Cassiopeia unzips it again when it reads the archive, which is the point of the format rather than a detour.

## The record shape

Inside the archive, each point is a `<Placemark>` carrying its attributes as `SchemaData` and its position as a `<Point>`:

```xml
<Placemark>
    <ExtendedData><SchemaData schemaUrl="#Yosemite_National_Park___Points_Of_Interest___Open_Data">
        <SimpleData name="OBJECTID">313</SimpleData>
        <SimpleData name="POINAME">Vernal Fall</SimpleData>
        <SimpleData name="POITYPE">Waterfall</SimpleData>
        <SimpleData name="UNITCODE">YOSE</SimpleData>
        <SimpleData name="ISSEASONALLYCLOSED"></SimpleData>
        <SimpleData name="LAT_DD_W84">37.7274313977158</SimpleData>
        <SimpleData name="LON_DD_W84">-119.543787490733</SimpleData>
    </SchemaData></ExtendedData>
    <Point><coordinates>-119.543787490733,37.7274313977158</coordinates></Point>
</Placemark>
```

KML attaches attributes in two ways, and Cassiopeia reads both: the `SchemaData`/`SimpleData` pairs used here, and plain `Data` elements. Either way the names become the record's properties, and every value arrives as text.

## The folder becomes a prefix

Every placemark in this file sits inside one `<Folder>`, named after the published layer. Cassiopeia nests each record under that folder's name, `snake_cased`, so a field is reached through that prefix rather than at the top level:

```json5
identity: {
    entityName: "{{ yosemite_national_park___points_of_interest___open_data.properties.OBJECTID }}",
}
```

The prefix is unwieldy here because the publisher's layer name is, and it is worth reading once: `Yosemite_National_Park___Points_Of_Interest___Open_Data` becomes `yosemite_national_park___points_of_interest___open_data`. A file whose folder is named `Camera Area` would use `camera_area`.

That prefix is what makes a KML with several folders usable: each folder's records stay in their own namespace instead of colliding in one flat record. This file has one folder, so every field goes through the one prefix. The [multi-folder example](../17-kml-folder-collections/example.md) routes four of them to four different mappings.

## Choose the entity identity

The obvious identity is the name, and it does not work. Ten placemarks are called `Upper Pines Campground Restroom`, eight are `Tuolumne Meadows Campground Restroom`: 553 points carry only 492 distinct names. Using the name would merge each of those groups into a single entity, exactly as the [ID collision example](../02-json-id-collision/example.md) shows.

`OBJECTID` is the park service's own identifier for a point, unique within the layer, so the mapping uses it and produces IDs such as `urn:ngsi-ld:ParkPointOfInterest:313`.

## The geometry, untouched

The record carries the position twice: as `LAT_DD_W84` and `LON_DD_W84` columns, and as the placemark's own `<Point>`. The mapping reads the geometry, not the columns:

```json5
location: {
    source: "{{ yosemite_national_park___points_of_interest___open_data.geometry }}",
    type: "GeoProperty",
    transformation: "geometry",
}
```

KML coordinates are WGS84 by definition, which is what NGSI-LD requires of a `GeoProperty` (ETSI GS CIM 009 v1.9.1, clause 4.7.1), so the geometry passes through as it stands. Rebuilding a point from two text columns would be more work and one more chance to swap latitude for longitude.

## A vocabulary kept, a flag made boolean

`POITYPE` is the park service's own vocabulary: `Waterfall`, `Trailhead`, `Campground`, `Historic Building`, `Bus Stop / Shuttle Stop`, and two dozen more. There is nothing to fold here, so the value is carried across as it stands. Folding a source vocabulary into fewer tokens, when a model demands it, is what the [conditionals example](../04-csv-conditionals/example.md) does.

`ISSEASONALLYCLOSED` is different. It reads `Yes` on 101 points, `No` on 84, `Unknown` on two, and is empty on the rest. Only two of those four states mean anything:

```json5
seasonallyClosed: {
    source: "{% if yosemite_national_park___points_of_interest___open_data.properties.ISSEASONALLYCLOSED == 'Yes' %}true{% elif yosemite_national_park___points_of_interest___open_data.properties.ISSEASONALLYCLOSED == 'No' %}false{% endif %}",
    type: "Property",
    transformation: "boolean",
}
```

A value outside the two matches no branch, the template produces nothing, and the attribute is left off that entity. An absent attribute says "not recorded"; `false` would say "open all year", which the source does not claim. Guarding an attribute this way is the technique from the [attribute guards example](../05-geojson-attribute-guards/example.md).

## The manifest

Open [manifest.json5](manifest.json5) for the run, and [park-point-of-interest.json5](park-point-of-interest.json5) for the full mapping:

```json5
{
    version: "v1",
    inputs: [
        {
            source: "data/yosemite-poi.kmz",
            mapping: "park-point-of-interest.json5",
            format: "kmz",
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

The format is `kmz` and the source is the archive; there is nothing to unzip by hand. `ParkPointOfInterest` is a model this example invents, so `fail-when-schema` writes the entities without a schema to check them against.

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
    ghcr.io/vela-tools/cassiopeia:latest \
    map \
    --manifest manifest.json5
```

From the repository root, the runner downloads the dataset, packages the archive, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 16
cargo run -- run 16 --runtime docker
```

The run writes `out/ParkPointOfInterest.json`, one array of 553 entities.

## Read the result

```json
[
    {
        "id": "urn:ngsi-ld:ParkPointOfInterest:313",
        "type": "ParkPointOfInterest",
        "name": {
            "type": "Property",
            "value": "Vernal Fall"
        },
        "location": {
            "type": "GeoProperty",
            "value": {
                "type": "Point",
                "coordinates": [
                    -119.543787490733,
                    37.7274313977158
                ]
            }
        },
        "category": {
            "type": "Property",
            "value": "Waterfall"
        },
        "park": {
            "type": "Property",
            "value": "YOSE"
        }
    },
    {
        "id": "urn:ngsi-ld:ParkPointOfInterest:3333",
        "type": "ParkPointOfInterest",
        "name": {
            "type": "Property",
            "value": "Glacier Point Ski Hut"
        },
        "location": {
            "type": "GeoProperty",
            "value": {
                "type": "Point",
                "coordinates": [
                    -119.573574349393,
                    37.728410362001
                ]
            }
        },
        "category": {
            "type": "Property",
            "value": "Hut"
        },
        "park": {
            "type": "Property",
            "value": "YOSE"
        },
        "seasonallyClosed": {
            "type": "Property",
            "value": true
        }
    }
]
```

Vernal Fall has no `seasonallyClosed` attribute because its source field is empty; the ski hut, which the park closes outside winter, carries `true`. Both keep the coordinates the placemark arrived with, and neither mentions the folder that named their prefix: namespacing shapes how a mapping reads a record, not what the entity ends up holding.
