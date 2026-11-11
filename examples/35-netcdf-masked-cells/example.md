# Skipping masked NetCDF grid cells

The [air quality forecast](../33-netcdf-grid-subset/example.md) had a value in every cell of its grid. A sea surface temperature analysis has none over land: the producer writes a fill value into every land cell, and the grid is mostly holes around the coast. This example maps NOAA's daily sea surface temperature analysis for the Adriatic to `SeaConditions` entities. The new idea is that the holes never reach the mapping. Cassiopeia masks a cell holding the fill value, and a cell masked in every data variable produces no record, so the mapping sees the sea cells only and needs no test for land.

## Get the data

NOAA's National Centers for Environmental Information (NCEI) produce the Optimum Interpolation Sea Surface Temperature analysis (OISST) v2.1, a daily global grid at a quarter of a degree that blends satellite, ship, and buoy measurements. NOAA's ERDDAP server publishes it with an aggregated time axis and cuts it down on request. The dataset's terms let the data be used and redistributed for free, without warranty.

```bash
mkdir -p data
curl --fail --location --output data/sst.nc "https://upwell.pfeg.noaa.gov/erddap/griddap/ncdcOisst21Agg_LonPM180.nc?sst%5B(2024-06-01T12:00:00Z)%5D%5B(0.0)%5D%5B(40):(46)%5D%5B(12):(20)%5D"
```

ERDDAP's grid queries put one pair of square brackets after the variable for each of its dimensions, here time, depth, latitude, and longitude, and the URL writes the brackets as `%5B` and `%5D`. The query asks for 1 June 2024 at the surface, from 40 to 46 degrees north and from 12 to 20 degrees east. ERDDAP snaps each bound to a grid point, and the file holds 25 latitudes by 33 longitudes, 825 cells, in about eleven kilobytes. The analysis for that day is the final one, released two weeks after the preliminary version, so the file is the same every time.

## Get the schema

`SeaConditions` is a published Smart Data Model. Download the catalog once:

```bash
cassiopeia sdm download
```

## The record shape

The file holds one data variable, with a fill value and a valid range:

```text
float sst(time, zlev, latitude, longitude) ;
    sst:_FillValue = -9.99f ;
    sst:valid_min = -3.f ;
    sst:valid_max = 45.f ;
    sst:units = "degree_C" ;
```

Of the 825 cells, 506 are land and hold -9.99. The cell at 45.875 degrees north and 14.375 degrees east, inland in Slovenia, is one of them. Cassiopeia counts a value as masked when it equals `_FillValue` or `missing_value`, or falls outside `valid_min` and `valid_max`. A masked value is left out of its record, and a cell masked in every data variable gets no record at all. This file has one data variable, so every land cell disappears. The southwest corner of the box is sea, west of Italy:

```json
{
    "latitude": 40.125,
    "longitude": 12.125,
    "time": "2024-06-01T12:00:00+00:00",
    "zlev": 0,
    "sst": 21.36
}
```

The depth axis, `zlev`, holds a single level at 0 metres with `positive = "down"`, so Cassiopeia takes it for a depth. CF does not say which surface a depth is measured below, so Cassiopeia reads that from the data variables: a soil quantity gives `depth_below_land` and a sea water quantity `depth_below_sea`. `sst` is a `sea_surface_temperature`, which is neither, so the level keeps its own name and value, `zlev` `0`. The mapping reads `latitude`, `longitude`, `time`, and `sst`, and ignores the level. The [NetCDF guide](https://vela-tools.github.io/cassiopeia/docs/guides/source-formats#gridded-records) covers masking in full.

## No test for land

Because a land cell never becomes a record, every record the mapping sees has a temperature, and the attribute reads it without a guard:

```json5
surfaceTemperature: {
    source: "{{ sst }}",
    type: "Property",
    transformation: "float",
    properties: {
        unitCode: {
            source: "CEL",
        },
    },
},
```

A file with several data variables behaves differently. A cell masked in one variable and valid in another still gets a record, without the masked field, so a template that does arithmetic on that field needs the `is defined` guard the [station example](../34-netcdf-station-time-series/example.md) uses.

The temperatures are stored as 32-bit floating point numbers. Cassiopeia writes each one as the shortest decimal that reads back as the same 32-bit value, so 21.36 stays `21.36` and the mapping needs no rounding. `CEL` is the UN/CEFACT Common Code for degree Celsius.

## One entity per sea cell

The day is fixed, so the cell alone is the identity. Six geohash characters describe a box about a kilometre across, much finer than the quarter degree grid:

```json5
identity: {
    entityName: "{{ geohash(lat=latitude, lon=longitude, precision=6) }}",
},
```

## Run it

A single source file, so no manifest:

```bash
cassiopeia map \
    --input data/sst.nc \
    --mapping sea-conditions.json5 \
    --type netcdf \
    --output out \
    --context none \
    --validation-mode fail-when-schema
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
    --input data/sst.nc \
    --mapping sea-conditions.json5 \
    --type netcdf \
    --output out \
    --context none \
    --validation-mode fail-when-schema
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 35
cargo run -- run 35 --runtime docker
```

This writes `out/SeaConditions.json`, 319 `SeaConditions` entities: the 825 cells of the box less its 506 land cells.

## Read the result

The cell in the Gulf of Trieste, off the Slovenian coast:

```json
{
    "id": "urn:ngsi-ld:SeaConditions:u21ew1",
    "type": "SeaConditions",
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                13.625,
                45.625
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2024-06-01T12:00:00Z",
        "valueType": "DateTime"
    },
    "surfaceTemperature": {
        "type": "Property",
        "value": 20.68,
        "unitCode": "CEL"
    }
}
```

And one at the southern edge of the box, off the coast of Albania:

```json
{
    "id": "urn:ngsi-ld:SeaConditions:srjuq8",
    "type": "SeaConditions",
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                19.625,
                40.125
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2024-06-01T12:00:00Z",
        "valueType": "DateTime"
    },
    "surfaceTemperature": {
        "type": "Property",
        "value": 21.36,
        "unitCode": "CEL"
    }
}
```

Every entity carries `dateObserved` with a `valueType` of `DateTime` and a temperature with its unit (ETSI GS CIM 009 v1.9.1, clause 4.5.2). No entity sits on land: the 506 cells holding the fill value never became records.
