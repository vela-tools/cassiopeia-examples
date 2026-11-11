# Mapping NetCDF station time series

The [previous example](../33-netcdf-grid-subset/example.md) read a NetCDF grid, one record per cell. Weather buoys and monitoring stations publish a different shape: a few fixed stations, each with its own series of observations. The CF conventions call it a time series feature type, and data servers such as ERDDAP write it straight from a query. This example maps a day of six NOAA weather buoys around Hawaii to one `WeatherObserved` entity per buoy. The new idea is how the station layout flattens: every observation becomes a record that carries its station's identifier and position, so a station identity and the `observedAt` of the [observedAt example](../07-csv-observed-at/example.md) apply unchanged.

## Get the data

NOAA's [National Data Buoy Center (NDBC)](https://www.ndbc.noaa.gov/) runs moored weather buoys from the Atlantic to the Pacific. This example takes six of them, moored in open water around Hawaii 150 to 200 nautical miles from the nearest island, which report the weather every ten minutes. The Environmental Research Division of NOAA Fisheries' Southwest Fisheries Science Center serves a standardized copy of NDBC's standard meteorological data from its [ERDDAP server](https://coastwatch.pfeg.noaa.gov/erddap/info/cwwcNDBCMet/index.html). NDBC is part of the National Weather Service, whose information is in the public domain, and the dataset's own licence lets the data be used and redistributed for free, without warranty.

ERDDAP builds a file from the query in the URL. The `.ncCF` suffix asks for NetCDF in the layout CF defines for station data, and the part before the first `&` names the columns. `station=~` takes a regular expression, here the six station numbers joined by `|`, which the URL writes with its quotes as `%22` and its bars as `%7C`. The rest selects one day:

```bash
mkdir -p data
curl --fail --location --output data/buoys.nc "https://coastwatch.pfeg.noaa.gov/erddap/tabledap/cwwcNDBCMet.ncCF?station,longitude,latitude,time,WD,WSPD,GST,BAR,ATMP,DEWP&station=~%2251000%7C51001%7C51002%7C51003%7C51004%7C51101%22&time%3E=2024-06-01T00:00:00Z&time%3C2024-06-02T00:00:00Z"
```

The day is 1 June 2024. A day that old comes from NDBC's quality-controlled yearly archive rather than its real-time feed, so the readings are the same every time: 864 observations, 144 from each of the buoys 51000 to 51004 and 51101, in just under forty kilobytes.

## Get the schema

`WeatherObserved` is a published Smart Data Model. Download the catalog once:

```bash
cassiopeia sdm download
```

## The record shape

The file declares `featureType = "TimeSeries"`. It holds each buoy once, along a `timeseries` dimension, and every observation once, along an `obs` dimension. The observations of one buoy are stored one after another, and `rowSize` says how many belong to each buoy, the contiguous ragged array of CF 1.11 section 9.3.3:

```text
char station(timeseries, station_strlen) ;
    station:cf_role = "timeseries_id" ;
float longitude(timeseries) ;
float latitude(timeseries) ;
int rowSize(timeseries) ;
    rowSize:sample_dimension = "obs" ;
double time(obs) ;
    time:units = "seconds since 1970-01-01T00:00:00Z" ;
float DEWP(obs) ;
    DEWP:_FillValue = -9999999.f ;
```

Cassiopeia works out which buoy each observation belongs to and gives every observation a record of its own, with the buoy's variables copied in. The time axis counts seconds since 1970 and names no calendar, so it is the standard calendar, and Cassiopeia decodes each value into the RFC 3339 timestamp it names. The first observation of 51000:

```json
{
    "latitude": 23.546,
    "longitude": -154.056,
    "time": "2024-06-01T00:00:00+00:00",
    "station": "51000",
    "WD": 74.0,
    "WSPD": 7.7,
    "GST": 10.1,
    "BAR": 1020.0,
    "ATMP": 23.5,
    "DEWP": 19.8
}
```

`WD` is the wind direction, `WSPD` and `GST` the wind speed and gust, `BAR` the pressure, `ATMP` the air temperature, and `DEWP` the dew point. A reading a buoy missed is stored as the variable's `_FillValue`, -9999999 for the float variables and 32767 for the wind direction, and is left out of the record. On this day 51101 reported no dew point and 51004 no pressure, so every record of 51101 lacks `DEWP`:

```json
{
    "latitude": 24.321,
    "longitude": -162.058,
    "time": "2024-06-01T00:00:00+00:00",
    "station": "51101",
    "WD": 70.0,
    "WSPD": 7.2,
    "GST": 8.6,
    "BAR": 1022.2,
    "ATMP": 24.4
}
```

The mapping reads every field shown. The [station time series guide](https://vela-tools.github.io/cassiopeia/docs/guides/source-formats#station-time-series) covers the other layouts CF allows, which give the same records.

## One entity per buoy

`station` is the variable the file marks with `cf_role = "timeseries_id"`, the identifier of each series, so it is the identity:

```json5
identity: {
    entityName: "{{ station }}",
},
```

All the records of a buoy now share one entity. Each reading carries an `observedAt` read from the record's `time`, so the readings rank by time instead of overwriting one another, and the latest one wins:

```json5
temperature: {
    source: "{{ ATMP }}",
    type: "Property",
    transformation: "float",
    properties: {
        unitCode: {
            source: "CEL",
        },
        observedAt: {
            source: "{{ time }}",
            type: "Property",
            transformation: "datetime",
        },
    },
},
```

`dateObserved`, which the model requires, carries the same `observedAt`, so it follows the readings to the last one of the day, at 23:50. The result is each buoy's current state. To keep the whole day as a series instead, add `--temporal-representation series`, as the observedAt example shows.

## From dew point to humidity

ERDDAP serves NDBC's readings in the units `WeatherObserved` expects, so most of them pass through with only their UN/CEFACT code beside them:

- **Wind speed and gust** are in metres per second (`MTS`).
- **Pressure** is in hectopascals (`A97`).
- **Air temperature and dew point** are in degrees Celsius (`CEL`), and **wind direction** in degrees from true north (`DD`).

The model also takes a relative humidity, as a ratio in `[0, 1]` (`C62`). The buoys report the dew point instead: the temperature to which the air would have to cool for its water vapour to condense. Together with the air temperature, the dew point fixes the relative humidity, and the Magnus formula converts one into the other. With the air temperature `T` and the dew point `Td` in degrees Celsius, the ratio is `exp(17.625 Td / (243.04 + Td) - 17.625 T / (243.04 + T))`, with the coefficients of Alduchov and Eskridge (1996):

```json5
relativeHumidity: {
    source: "{% if ATMP is defined and DEWP is defined %}{% set exponent = 17.625 * DEWP / (243.04 + DEWP) - 17.625 * ATMP / (243.04 + ATMP) %}{{ exponent | exp | round(precision=4) }}{% endif %}",
    type: "Property",
    transformation: "float",
    properties: {
        unitCode: {
            source: "C62",
        },
        observedAt: {
            source: "{{ time }}",
            type: "Property",
            transformation: "datetime",
        },
    },
},
```

The `{% if ... is defined %}` guard matters here. A missed reading is absent from the record, and arithmetic on an absent field fails the template. Without the guard, the template fails for 51101, and the run drops the attribute with a warning that names the missing `DEWP`. With it, 51101 gives no humidity and the run stays clean. A plain reference such as `{{ DEWP }}` or `{{ BAR }}` needs no guard: an absent field resolves to nothing, and the attribute is left out of that record.

## Run it

A single source file, so no manifest:

```bash
cassiopeia map \
    --input data/buoys.nc \
    --mapping weather.json5 \
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
    --input data/buoys.nc \
    --mapping weather.json5 \
    --type netcdf \
    --output out \
    --context none \
    --validation-mode fail-when-schema
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 34
cargo run -- run 34 --runtime docker
```

This writes `out/WeatherObserved.json`, six `WeatherObserved` entities made from the 864 records, one for each buoy.

## Read the result

51000, about 250 nautical miles northeast of Honolulu, at its last reading of the day:

```json
{
    "id": "urn:ngsi-ld:WeatherObserved:51000",
    "type": "WeatherObserved",
    "stationCode": {
        "type": "Property",
        "value": "51000"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -154.056,
                23.546
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2024-06-01T23:50:00Z",
        "valueType": "DateTime",
        "observedAt": "2024-06-01T23:50:00Z"
    },
    "temperature": {
        "type": "Property",
        "value": 23.7,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "CEL"
    },
    "dewPoint": {
        "type": "Property",
        "value": 19.0,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "CEL"
    },
    "relativeHumidity": {
        "type": "Property",
        "value": 0.7497,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "C62"
    },
    "atmosphericPressure": {
        "type": "Property",
        "value": 1019.9,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "A97"
    },
    "windDirection": {
        "type": "Property",
        "value": 62,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "DD"
    },
    "windSpeed": {
        "type": "Property",
        "value": 8.5,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "MTS"
    },
    "gustSpeed": {
        "type": "Property",
        "value": 10.3,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "MTS"
    }
}
```

51101, about 186 nautical miles northwest of Kauai, at the same time. It reported no dew point all day, so its entity has neither `dewPoint` nor `relativeHumidity`:

```json
{
    "id": "urn:ngsi-ld:WeatherObserved:51101",
    "type": "WeatherObserved",
    "stationCode": {
        "type": "Property",
        "value": "51101"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -162.058,
                24.321
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2024-06-01T23:50:00Z",
        "valueType": "DateTime",
        "observedAt": "2024-06-01T23:50:00Z"
    },
    "temperature": {
        "type": "Property",
        "value": 24.1,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "CEL"
    },
    "atmosphericPressure": {
        "type": "Property",
        "value": 1022.7,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "A97"
    },
    "windDirection": {
        "type": "Property",
        "value": 75,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "DD"
    },
    "windSpeed": {
        "type": "Property",
        "value": 7.3,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "MTS"
    },
    "gustSpeed": {
        "type": "Property",
        "value": 9.5,
        "observedAt": "2024-06-01T23:50:00Z",
        "unitCode": "MTS"
    }
}
```

`observedAt` sits beside each value as the temporal property that records when the reading was observed (ETSI GS CIM 009 v1.9.1, clause 4.8), and `unitCode` beside it gives the reading's unit (clause 4.5.2). 51000's air temperature of 23.7 degrees and dew point of 19.0 degrees give the relative humidity 0.7497. The entity of 51004, which reported no pressure, has every attribute except `atmosphericPressure`.
