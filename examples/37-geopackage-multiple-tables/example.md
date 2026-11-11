# Mapping several GeoPackage tables with one manifest

A GeoPackage is a single SQLite file that can hold many tables. This example maps the German administrative units of the Federal Agency for Cartography and Geodesy (BKG) to three NGSI-LD types, `FederalState`, `District`, and `Municipality`, and links each municipality to its district and each district to its state.

The idea this example adds to [mapping a one-table GeoPackage](../36-geopackage-lau-municipalities/example.md) is that one GeoPackage is several collections. Each table is bound to its own mapping in a manifest, and the tables no mapping names are never opened. It builds on [routing KML folders to their own mappings](../17-kml-folder-collections/example.md) and on [relationships between entities](../10-json-relationships/example.md).

> [!WARNING]
> This example reads a GeoPackage, which needs SQLite. The prebuilt release binaries do not include it (`--no-default-features`) and report a GeoPackage as unsupported, so they cannot run this example; the container image can. A source build with the default features compiles SQLite itself, as described in [Getting started](https://vela-tools.github.io/cassiopeia/docs/guides/getting-started#geopackage-and-sqlite).

## Get the data

The BKG publishes the administrative units at a scale of 1:5,000,000 as the product VG5000. The archive is 10 MB and holds the documentation and one GeoPackage, `DE_VG5000.gpkg`, which is about 23 MB. The data is licensed under the Data licence Germany, attribution, version 2.0 (`dl-de/by-2-0`): commercial use is allowed with the source named, here the BKG. The file is republished each year and sometimes corrected in between, so a later download can hold a few more or fewer units than quoted here.

```bash
curl --fail --location \
    --output data/vg5000.zip \
    https://daten.gdz.bkg.bund.de/produkte/vg/vg5000_0101/aktuell/vg5000_01-01.utm32s.gpkg.ebenen.zip
unzip -p data/vg5000.zip vg5000_ebenen_0101/DE_VG5000.gpkg > data/vg5000.gpkg
```

The `unzip -p` step writes the one member this example reads to `data/vg5000.gpkg`. Cassiopeia reads a GeoPackage file, not an archive of one.

## The record shape

The GeoPackage lists forty tables and views in its `gpkg_contents` catalog. Seven are feature tables with a geometry column, seven are attribute tables of codes, and twenty-six are views over them. Three feature tables hold the administrative levels this example maps:

| Table | Rows | Holds |
| --- | --- | --- |
| `vg5000_lan` | 25 | The 16 federal states, and 9 more rows for the same states without structure |
| `vg5000_krs` | 401 | The districts |
| `vg5000_gem` | 10,939 | The municipalities |

Cassiopeia emits one record per row. The table's columns sit under `properties`, named exactly as in the table, and the geometry sits beside them. A row of `vg5000_krs` becomes this record, with the geometry shortened to its first ring's first positions:

```json
{
    "properties": {
        "id": 1,
        "OBJID": "DEBKGVG500000CH6",
        "BEGINN": "2019-10-04T00:00:00.000Z",
        "ADE": 4,
        "GF": 9,
        "BSG": 1,
        "LKZ": "SH",
        "ARS": "01001",
        "AGS": "01001",
        "SDV_ARS": "010010000000",
        "GEN": "Flensburg",
        "BEZ": "Kreisfreie Stadt",
        "IBZ": 40,
        "BEM": "--",
        "NBD": "ja",
        "SN_L": "01",
        "SN_R": "0",
        "SN_K": "01",
        "SN_V1": "00",
        "SN_V2": "00",
        "SN_G": "000",
        "FK_S3": "R",
        "NUTS": "DEF01",
        "ARS_0": "010010000000",
        "AGS_0": "01001000",
        "WSK": "2008-01-01T00:00:00.000Z"
    },
    "geometry": {
        "type": "MultiPolygon",
        "coordinates": [[[[9.49234083982587, 54.822318178144336], [9.464208618623465, 54.822370643863536], [9.435742295515196, 54.79254668043326]]]]
    }
}
```

The mapping reads the name `GEN`, the designation `BEZ`, the `NUTS` code, the start of validity `BEGINN`, the regional key `ARS`, and the state, region, and district keys `SN_L`, `SN_R`, and `SN_K`. It ignores the other columns. The columns are declared `DATETIME`, so the ingestor reads them as ISO 8601 timestamps and the mapping can hand them straight to the `datetime` transformation.

The geometry is the one thing the file does not store this way. Its table declares the coordinate reference system EPSG:25832, ETRS89 with UTM zone 32N, whose coordinates are metres. Cassiopeia reads that definition from the file's `gpkg_spatial_ref_sys` table and reprojects every geometry to the WGS 84 longitude and latitude that a `GeoProperty` requires (ETSI GS CIM 009 v1.9.1, clause 4.7). The mapping does nothing for it.

## One manifest, three tables

An input with a `mappings` list reads only the tables it names. Each `collection` is a table name as `gpkg_contents` lists it:

```json5
mappings: [
    {
        collection: "vg5000_lan",
        mapping: "federal-state.json5",
    },
    {
        collection: "vg5000_krs",
        mapping: "district.json5",
    },
    {
        collection: "vg5000_gem",
        mapping: "municipality.json5",
    },
],
```

The other thirty-seven tables and views are never opened. That is more than tidiness. `gpkg_contents` lists the views as well as the tables, and Cassiopeia identifies each row by its table's integer primary key, which a view lacks. With a single `mapping`, Cassiopeia reads every record table, skips each of the twenty-six views with an `ingest-table-unreadable` warning, and carries on. Naming the three tables avoids both the warnings and the reading.

## Keeping one row per state

The state table holds each state twice. Sixteen rows carry `GF` 9, with structure, and nine carry `GF` 8, without structure, according to the file's `v_werte_gf` lookup. Each of the nine shares its `ARS` with one of the sixteen, so they would mint the same identifiers twice, and the identity keeps only the `GF` 9 rows:

```json5
identity: {
    entityName: "{% if properties.GF == 9 %}{{ properties.ARS }}{% endif %}",
},
```

A guarded template that renders nothing names no entity, so Cassiopeia skips the record and counts one `expander-urn-ungeneratable` warning per skipped row. The run therefore reports nine warnings by design, and writes sixteen `FederalState` entities. The district and municipality tables hold `GF` 9 only, so they need no guard.

## Linking the levels

The regional key `ARS` of a unit is its identifier at every level, and each level's key starts with the key above it. A state's key is two digits, a district's is five, and a municipality's is twelve. The district mapping mints its entities from `ARS`:

```json5
identity: {
    entityName: "{{ properties.ARS }}",
},
```

A relationship builds its target's identifier from its `source` and `target.entity`, so a source that reproduces the target's `ARS` reaches the same entity. A district knows its state through `SN_L`, the state's two-digit key:

```json5
federalState: {
    source: "{{ properties.SN_L }}",
    type: "Relationship",
    target: {
        entity: "FederalState",
    },
},
```

A municipality has no column holding its district's key, but the first five digits of its own key are three columns: the state `SN_L`, the administrative region `SN_R`, and the district `SN_K`. A template joins them:

```json5
district: {
    source: "{{ properties.SN_L }}{{ properties.SN_R }}{{ properties.SN_K }}",
    type: "Relationship",
    target: {
        entity: "District",
    },
},
```

For Flensburg, whose columns hold `01`, `0`, and `01`, the source is `01001`, so the municipality points at `urn:ngsi-ld:District:01001`. Joining only `SN_L` and `SN_K` would give `0101`, which names no district. The run reports no error for a link to a missing entity, so check a link against the target's identifier rather than trusting that it resolves.

## Run it

The three tables come from one input with three mappings, so the run takes a manifest:

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
    ghcr.io/vela-tools/cassiopeia:v1.2.0 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner downloads the dataset, unpacks the GeoPackage, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 37
cargo run -- run 37 --runtime docker
```

The run writes `out/FederalState.json` with 16 entities, `out/District.json` with 401, and `out/Municipality.json` with 10,939, about 26 MB for the municipalities. Every entity is written without a schema check, because these are plain data models with no published schema, and the run counts that once per entity as `schema-absent`.

## Read the result

A district carries its geometry, its start of validity, and the link to its state. The ring is shortened to its first three positions:

```json
{
    "id": "urn:ngsi-ld:District:01001",
    "type": "District",
    "name": {
        "type": "Property",
        "value": "Flensburg"
    },
    "designation": {
        "type": "Property",
        "value": "Kreisfreie Stadt"
    },
    "nutsCode": {
        "type": "Property",
        "value": "DEF01"
    },
    "validFrom": {
        "type": "Property",
        "value": "2019-10-04T00:00:00Z",
        "valueType": "DateTime"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "MultiPolygon",
            "coordinates": [[[[9.49234083982587, 54.822318178144336], [9.464208618623465, 54.822370643863536], [9.435742295515196, 54.79254668043326]]]]
        }
    },
    "federalState": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:FederalState:01",
        "objectType": "FederalState"
    }
}
```

The coordinates are longitude then latitude in degrees, near 9.5 east and 54.8 north, which is Flensburg on the Danish border. The source stored them as UTM metres. The `datetime` transformation wrote `validFrom` with a `valueType` of `DateTime` (ETSI GS CIM 009 v1.9.1, clause 4.5.2), so a broker reads it as a time and not as text.

The municipality of the same name is a separate entity with the same outline, because Flensburg is a district-free city. It points one level up:

```json
{
    "id": "urn:ngsi-ld:Municipality:010010000000",
    "type": "Municipality",
    "district": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:District:01001",
        "objectType": "District"
    }
}
```

The entity also carries the five attributes of the district above, which this excerpt leaves out. Every one of the 10,939 municipalities and 401 districts points at an entity the run wrote.
