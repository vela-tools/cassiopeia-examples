# Mapping a GeoPackage of European municipalities

A GeoPackage is a single SQLite file that holds geographic tables. This example maps Eurostat's local administrative units, the municipalities of 34 European countries, to `LocalAdministrativeUnit` entities with their population, area, and outline.

The idea this example adds is that a GeoPackage carries its own coordinate reference system. This file stores its geometry as metres on the European equal-area grid, EPSG:3035, and Cassiopeia reprojects every outline to the WGS 84 longitude and latitude a `GeoProperty` needs, with nothing in the mapping to ask for it. A shapefile does the same from its `.prj`, as [example 15](../15-shapefile-enum-decoding/example.md) describes for a file already in WGS 84. [The next example](../37-geopackage-multiple-tables/example.md) takes on a GeoPackage with many tables.

## Get the data

Eurostat's GISCO service publishes the local administrative units (LAU) once a year. The 2024 edition in the EPSG:3035 projection is a single file of 78 MB, and Eurostat offers no per-country cut of it, so the example reads all of Europe. GISCO grants a non-exclusive right to use the data; commercial use needs a EuroGeographics licence, and the boundaries stay their copyright, as in [example 31](../31-geojson-geometry-conversion/example.md).

```bash
curl --fail --location \
    --output data/lau.gpkg \
    https://gisco-services.ec.europa.eu/distribution/v2/lau/gpkg/LAU_RG_01M_2024_3035.gpkg
```

The file holds 97,987 units. Mapping all of them writes a 374 MB file and, on a debug build, reaches a peak of about 1.8 GiB of memory, so run it on a machine with room to spare.

## The record shape

A GeoPackage lists its record tables in a `gpkg_contents` catalog, and this one lists a single feature table. Cassiopeia emits one record per row. The table's columns sit under `properties`, named exactly as in the table, and the geometry sits beside them. The row for Ljubljana becomes this record, with its geometry shown below:

```json
{
    "properties": {
        "fid": 95246,
        "GISCO_ID": "SI_061",
        "CNTR_CODE": "SI",
        "LAU_NAME": "Ljubljana",
        "POP_2024": 297575.0,
        "POP_DENS_2024": 1081.9004345,
        "AREA_KM2": 275.04841528,
        "YEAR": 2024
    },
    "geometry": {
        "type": "Polygon",
        "coordinates": [[[14.51375300055804, 46.14414149986253], [14.506669466079375, 46.145364725214954]]]
    }
}
```

The primary key comes first under its own column name, `fid`. Cassiopeia converts each value by the type its column declares: `TEXT` stays a string and `REAL` becomes a number, which is why the population arrives as `297575.0`. The ring in the record is shortened to its first two positions. The mapping reads the identifier, the name, the country, the population, the density, and the area, and ignores `fid` and `YEAR`.

## The coordinate reference system

The GeoPackage stores the table's coordinate reference system as the `srs_id` of its geometry column, and defines it in `gpkg_spatial_ref_sys`. For this file it is EPSG:3035, whose coordinates are metres. Cassiopeia reads that definition and reprojects every geometry to EPSG:4326 as it reads the row, so the Ljubljana ring above is already in degrees: about 14.51 east and 46.14 north. A `GeoProperty` takes WGS 84 longitude and latitude (ETSI GS CIM 009 v1.9.1, clause 4.7, which follows RFC 7946 clause 4), so the mapping passes the geometry straight through:

```json5
location: {
    source: "{{ geometry }}",
    type: "GeoProperty",
    transformation: "geometry",
},
```

A table already in EPSG:4326 is emitted unchanged. A table whose definition is `undefined` is assumed to be EPSG:4326 as well, with a warning, and a coordinate outside WGS 84 is refused when the attribute is built.

## Identity

The GISCO identifier is the country code and the unit's national code, so it is unique across the 97,987 units and the identity reads it as it is:

```json5
identity: {
    entityName: "{{ properties.GISCO_ID }}",
},
```

Ljubljana becomes `urn:ngsi-ld:LocalAdministrativeUnit:SI_061`.

## A figure the source does not have

The file stores `0` for a unit it has no population for. Every unit of France, Spain, and Albania holds a `0`, along with 203 German ones, 43,859 units in all, and the page for the dataset says the files include population "where available". Written as a number, a `0` would say that nobody lives in the unit. The mapping reads a population only when it is positive:

```json5
population: {
    source: "{% if properties.POP_2024 > 0 %}{{ properties.POP_2024 }}{% endif %}",
    type: "Property",
    transformation: "integer",
},
```

A guarded template that renders nothing gives no attribute, and the entity is still written. The same guard covers the density. The 203 German units may be a real zero, because a few unincorporated areas have no residents; the file does not say which, so the mapping leaves them off with the rest. The `integer` transformation then writes `297575.0` as `297575`.

The area needs no guard and carries a `unitCode` of `KMK`, the UN/CEFACT Common Code for square kilometre (ETSI GS CIM 009 v1.9.1, clause 4.5.2.2).

## Run it

This is a single source file, so the run needs no manifest. `--type geopackage` states the format, though Cassiopeia would also recognise the file from its SQLite header:

```bash
cassiopeia map \
    --input data/lau.gpkg \
    --mapping lau.json5 \
    --type geopackage \
    --output out \
    --context none \
    --validation-mode fail-when-schema
```

The same run in a container mounts this directory at `/data` and makes it the working directory, so the paths do not change. For Podman, replace `docker` with `podman` and drop the `--user` line: rootless Podman already maps the container's root to your user.

```bash
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "$PWD:/data" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.1.1 \
    map \
    --input data/lau.gpkg \
    --mapping lau.json5 \
    --type geopackage \
    --output out \
    --context none \
    --validation-mode fail-when-schema
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 36
cargo run -- run 36 --runtime docker
```

The run writes `out/LocalAdministrativeUnit.json` with 97,987 entities. `LocalAdministrativeUnit` is a plain data model with no published schema, so every entity is written without a schema check and the run counts that once per entity as `schema-absent`.

## Read the result

Ljubljana carries its figures and its outline. The ring is shortened to its first two positions:

```json
{
    "id": "urn:ngsi-ld:LocalAdministrativeUnit:SI_061",
    "type": "LocalAdministrativeUnit",
    "name": {
        "type": "Property",
        "value": "Ljubljana"
    },
    "countryCode": {
        "type": "Property",
        "value": "SI"
    },
    "population": {
        "type": "Property",
        "value": 297575
    },
    "populationDensity": {
        "type": "Property",
        "value": 1081.9004345
    },
    "area": {
        "type": "Property",
        "value": 275.04841528,
        "unitCode": "KMK"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Polygon",
            "coordinates": [[[14.51375300055804, 46.14414149986253], [14.506669466079375, 46.145364725214954]]]
        }
    }
}
```

The source stored the outline as EPSG:3035 metres and the mapping never mentions a projection. A French commune of the same file has the same shape without `population` and `populationDensity`, because the file holds no figure for it.
