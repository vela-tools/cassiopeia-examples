# Mapping a NetCDF air quality forecast grid

NetCDF is how air quality and ocean models, climate reanalyses, and data servers such as THREDDS and ERDDAP publish gridded data. Cassiopeia reads a NetCDF grid into the same records as a GRIB one, one per cell, so the mapping style of the [GRIB2 example](../19-grib2-byte-range/example.md) carries over. This example maps the Finnish Meteorological Institute's SILAM air quality forecast over Slovenia to `AirQualityForecast` entities. It adds two ideas: asking the data server for only the part of the grid the run needs, and converting the molar concentrations a chemistry model counts in into the micrograms per cubic metre the model expects. The [next example](../34-netcdf-station-time-series/example.md) reads station data from a NetCDF file instead of a grid.

## Get the data

SILAM is the chemistry transport model of the [Finnish Meteorological Institute (FMI)](https://silam.fmi.fi/), and FMI publishes its forecasts as open data under CC BY 4.0. The European run covers the continent on a 0.1 degree grid, at ten layers from the ground up, and FMI serves it from a THREDDS data server. THREDDS runs a NetCDF Subset Service: the query names the variables, the box, the layer, and the time, and the server cuts the file down to them before anything is sent.

```bash
mkdir -p data
curl --fail --location --output data/forecast.nc "https://thredds.silam.fmi.fi/thredds/ncss/grid/silam_europe_v6_1/silam_europe_v6_1_best.ncd?var=cnc_O3_gas&var=cnc_NO2_gas&var=cnc_PM2_5&north=46.9&south=45.4&west=13.3&east=16.6&vertCoord=12.5&time=present&accept=netcdf"
```

The query asks for ozone, nitrogen dioxide, and fine particulate matter (`var`), over a box around Slovenia (`north`, `south`, `west`, `east`), in the lowest model layer, whose midpoint sits 12.5 metres above the ground (`vertCoord`), at the forecast hour nearest the request (`time=present`). That is a file of about ten kilobytes holding a grid of 34 longitudes by 16 latitudes. Each run of the example downloads whichever forecast hour is current, so the values change from run to run while the grid stays the same.

## Get the schema

`AirQualityForecast` is a published Smart Data Model. Download the catalog once:

```bash
cassiopeia sdm download
```

## The record shape

Cassiopeia reads one record for every cell of the grid. The cell over Ljubljana looks like this:

```json
{
    "latitude": 46.05,
    "longitude": 14.55,
    "time": "2026-11-10T09:00:00+00:00",
    "height": 12.5,
    "time_run": "2026-11-10T01:00:00+00:00",
    "cnc_O3_gas": 2.0954758e-8,
    "cnc_NO2_gas": 7.4505806e-7,
    "cnc_PM2_5": 5.800694e-8
}
```

Cassiopeia recognises `latitude`, `longitude`, and `time` by their CF attributes and gives them the same keys whatever the file calls them. It decodes the time axis, stored as hours since an epoch, into an RFC 3339 timestamp. `height` is the model layer the query picked, and `time_run` is when the forecast run started. THREDDS writes the run time beside every forecast hour, and Cassiopeia keeps it under the variable's own name. The data variables keep the names SILAM gave them. The mapping reads `latitude`, `longitude`, `time`, and the three concentrations, and ignores `height` and `time_run`. The [NetCDF guide](https://vela-tools.github.io/cassiopeia/docs/guides/source-formats#netcdf) lists everything a record can hold.

SILAM stores its coordinates and concentrations as 32-bit floating point numbers, and Cassiopeia writes each one as the shortest decimal that reads back as the same 32-bit value, so the cell centre above reads `46.05`. Not every centre in the file sits exactly on the grid's decimals, though: seven latitudes and three longitudes hold the 32-bit neighbour of their decimal, so 45.65 reads `45.649998` and 14.65 reads `14.650001`.

## One entity per grid cell

The download holds a single forecast hour, so the cell alone identifies a forecast. The mapping names it with a geohash of the cell's coordinates, as the [GRIB1 example](../18-grib1-derived-values/example.md) does:

```json5
identity: {
    entityName: "{{ geohash(lat=latitude, lon=longitude, precision=7) }}",
},
```

Seven geohash characters describe a box about 150 metres across, much finer than the 0.1 degree grid, so no two cells share an identifier. Because the identifier does not include the hour, the same cell keeps the same entity from one forecast run to the next.

The location rounds the coordinates back to the two decimals the grid is laid out in:

```json5
location: {
    source: [
        "{{ longitude | round(precision=2) }}",
        "{{ latitude | round(precision=2) }}",
    ],
    type: "GeoProperty",
    transformation: "point",
},
```

## From moles to micrograms

`AirQualityForecast` gives concentrations in micrograms per cubic metre. SILAM counts the gases in moles per cubic metre, and each variable states the molar mass the model used:

```text
cnc_O3_gas:units = "mole/m3" ;
cnc_O3_gas:molar_mass = "0.0480000            kg/mole" ;
cnc_NO2_gas:molar_mass = "0.0460000            kg/mole" ;
cnc_PM2_5:units = "kg/m3" ;
```

Multiplying moles by the molar mass gives kilograms, and a kilogram is 10⁹ micrograms. Ozone is therefore the concentration times 0.048 times 10⁹:

```json5
o3: {
    source: "{% set micrograms = cnc_O3_gas * 0.048 * 1000000000 %}{{ micrograms | round(precision=1) }}",
    type: "Property",
    transformation: "float",
    properties: {
        unitCode: {
            source: "GQ",
        },
    },
},
```

The `{% set %}` names the product first, so `round` applies to the whole of it. Nitrogen dioxide is the same with 0.046. Fine particulate matter is a mixture with no single molar mass, so SILAM already gives it as a mass in kilograms per cubic metre, and only the factor of 10⁹ remains. `GQ` is the UN/CEFACT Common Code for microgram per cubic metre.

## The validity of a forecast

`AirQualityForecast` states the period a forecast holds for with `validFrom` and `validTo`. SILAM does not state an averaging period for its concentrations: the variables carry no `cell_methods` attribute and the time axis has no bounds. With neither, the mapping reads each value as a forecast for the instant its time names, and writes that instant as both ends of the validity:

```json5
validFrom: {
    source: "{{ time }}",
    type: "Property",
    transformation: "datetime",
},
validTo: {
    source: "{{ time }}",
    type: "Property",
    transformation: "datetime",
},
```

## Run it

A single source file, so no manifest:

```bash
cassiopeia map \
    --input data/forecast.nc \
    --mapping air-quality-forecast.json5 \
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
    --input data/forecast.nc \
    --mapping air-quality-forecast.json5 \
    --type netcdf \
    --output out \
    --context none \
    --validation-mode fail-when-schema
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 33
cargo run -- run 33 --runtime docker
```

This writes `out/AirQualityForecast.json`, 544 `AirQualityForecast` entities, one for each cell of the 34 by 16 grid.

## Read the result

The cell over Ljubljana, from the forecast for 09:00 UTC on 10 November 2026. A later run gives the same entity with the values of a later hour:

```json
{
    "id": "urn:ngsi-ld:AirQualityForecast:u24mgp4",
    "type": "AirQualityForecast",
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                14.55,
                46.05
            ]
        }
    },
    "validFrom": {
        "type": "Property",
        "value": "2026-11-10T09:00:00Z",
        "valueType": "DateTime"
    },
    "validTo": {
        "type": "Property",
        "value": "2026-11-10T09:00:00Z",
        "valueType": "DateTime"
    },
    "o3": {
        "type": "Property",
        "value": 1.0,
        "unitCode": "GQ"
    },
    "no2": {
        "type": "Property",
        "value": 34.3,
        "unitCode": "GQ"
    },
    "pm25": {
        "type": "Property",
        "value": 58.0,
        "unitCode": "GQ"
    }
}
```

`validFrom` and `validTo` carry a `valueType` of `DateTime`, so a broker compares them as times rather than as text (ETSI GS CIM 009 v1.9.1, clause 4.5.2). Each concentration carries its unit as a `unitCode` beside the value, also under clause 4.5.2.
