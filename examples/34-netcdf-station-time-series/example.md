# Mapping NetCDF station time series

The [previous example](../33-netcdf-grid-subset/example.md) read a NetCDF grid, one record per cell. Weather buoys and monitoring stations publish a different shape: a few fixed stations, each with its own series of observations. The CF conventions call it a time series feature type, and data servers such as ERDDAP write it straight from a query. This example maps a day of the Irish Weather Buoy Network to one `WeatherObserved` entity per buoy. The new idea is how the station layout flattens: every observation becomes a record that carries its station's identifier and position, so a station identity and the `observedAt` of the [observedAt example](../07-csv-observed-at/example.md) apply unchanged.

## Get the data

The [Marine Institute](https://www.marine.ie/) runs the Irish Weather Buoy Network with Met Éireann: five moored buoys around the coast of Ireland that report hourly. The institute serves the readings from its [ERDDAP server](https://erddap.marine.ie/erddap/) under CC BY 4.0. ERDDAP builds a file from the query in the URL. The `.ncCF` suffix asks for NetCDF in the layout CF defines for station data, the part before the first `&` names the columns, and the rest selects one day:

```bash
mkdir -p data
curl --fail --location --output data/buoys.nc "https://erddap.marine.ie/erddap/tabledap/IWBNetwork.ncCF?station_id,longitude,latitude,time,AtmosphericPressure,AirTemperature,RelativeHumidity,WindDirection,WindSpeed,Gust&time%3E=2024-06-01T00:00:00Z&time%3C2024-06-02T00:00:00Z"
```

The day is 1 June 2024, so the file is the same every time: 115 hourly observations from the five buoys, M2 to M6, in about twenty kilobytes.

## Get the schema

`WeatherObserved` is a published Smart Data Model. Download the catalog once:

```bash
cassiopeia sdm download
```

## The record shape

The file declares `featureType = "TimeSeries"`. It holds each buoy once, along a `timeseries` dimension, and every observation once, along an `obs` dimension. The observations of one buoy are stored one after another, and `rowSize` says how many belong to each buoy, the contiguous ragged array of CF 1.11 section 9.3.3:

```text
char station_id(timeseries, station_id_strlen) ;
    station_id:cf_role = "timeseries_id" ;
double longitude(timeseries) ;
double latitude(timeseries) ;
int rowSize(timeseries) ;
    rowSize:sample_dimension = "obs" ;
double time(obs) ;
double AirTemperature(obs) ;
```

Cassiopeia works out which buoy each observation belongs to and gives every observation a record of its own, with the buoy's variables copied in. The first observation of M2:

```json
{
    "latitude": 53.4836,
    "longitude": -5.4302,
    "time": "2024-06-01T00:00:00+00:00",
    "station_id": "M2",
    "AtmosphericPressure": 1027.869,
    "AirTemperature": 11.973,
    "RelativeHumidity": 83.691,
    "WindDirection": 349.0,
    "WindSpeed": 10.929,
    "Gust": 14.345
}
```

A reading a buoy missed is stored as the `_FillValue`, -999, and is left out of the record. The mapping reads every field shown. The [station time series guide](https://vela-tools.github.io/cassiopeia/docs/guides/source-formats#station-time-series) covers the other layouts CF allows, which give the same records.

The file's time axis declares `calendar = "julian"`. Between 1901 and 2099 the Julian and Gregorian calendars have the same leap years, so counted from a 1970 epoch both give the same dates, and the timestamps come out as the day the query asked for.

## One entity per buoy

`station_id` is the variable the file marks with `cf_role = "timeseries_id"`, the identifier of each series, so it is the identity:

```json5
identity: {
    entityName: "{{ station_id }}",
},
```

All the hourly records of a buoy now share one entity. Each reading carries an `observedAt` read from the record's `time`, so the readings rank by time instead of overwriting one another, and the latest one wins:

```json5
temperature: {
    source: "{{ AirTemperature }}",
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

`dateObserved`, which the model requires, carries the same `observedAt`, so it follows the readings to the latest hour. The result is each buoy's current state. To keep the whole day as a series instead, add `--temporal-representation series`, as the observedAt example shows.

## Converting the units

The network reports in the units mariners use, and `WeatherObserved` expects SI units and ratios:

- **Wind speed and gust** are in knots. A knot is 1852 metres an hour, so the mapping multiplies by 1852 and divides by 3600 to get metres per second (`MTS`).
- **Relative humidity** is a percentage, and the model wants a ratio in `[0, 1]` (`C62`).
- **Pressure** is in millibars, which are hectopascals (`A97`), so it passes through.
- **Air temperature** is already in degrees Celsius (`CEL`), and **wind direction** in degrees (`DD`).

```json5
windSpeed: {
    source: "{% if WindSpeed is defined %}{% set metres = WindSpeed * 1852 / 3600 %}{{ metres | round(precision=2) }}{% endif %}",
    type: "Property",
    transformation: "float",
    properties: {
        unitCode: {
            source: "MTS",
        },
        observedAt: {
            source: "{{ time }}",
            type: "Property",
            transformation: "datetime",
        },
    },
},
```

The `{% if ... is defined %}` guard matters here. A missed reading is absent from the record, and arithmetic on an absent field fails the template. A plain reference such as `{{ AirTemperature }}` needs no guard: an absent field resolves to nothing, and the attribute is left out of that record.

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
    ghcr.io/vela-tools/cassiopeia:v1.1.1 \
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

This writes `out/WeatherObserved.json`, five `WeatherObserved` entities made from the 115 records, one for each buoy.

## Read the result

M2, off the east coast in the Irish Sea, at its last reading of the day:

```json
{
    "id": "urn:ngsi-ld:WeatherObserved:M2",
    "type": "WeatherObserved",
    "stationCode": {
        "type": "Property",
        "value": "M2"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -5.4302,
                53.4836
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2024-06-01T23:00:00Z",
        "valueType": "DateTime",
        "observedAt": "2024-06-01T23:00:00Z"
    },
    "temperature": {
        "type": "Property",
        "value": 11.875,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "CEL"
    },
    "relativeHumidity": {
        "type": "Property",
        "value": 0.8506,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "C62"
    },
    "atmosphericPressure": {
        "type": "Property",
        "value": 1031.018,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "A97"
    },
    "windDirection": {
        "type": "Property",
        "value": 3,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "DD"
    },
    "windSpeed": {
        "type": "Property",
        "value": 6.27,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "MTS"
    },
    "gustSpeed": {
        "type": "Property",
        "value": 7.79,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "MTS"
    }
}
```

M6, far out in the Atlantic west of Ireland, at the same hour:

```json
{
    "id": "urn:ngsi-ld:WeatherObserved:M6",
    "type": "WeatherObserved",
    "stationCode": {
        "type": "Property",
        "value": "M6"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -15.88135,
                53.07482
            ]
        }
    },
    "dateObserved": {
        "type": "Property",
        "value": "2024-06-01T23:00:00Z",
        "valueType": "DateTime",
        "observedAt": "2024-06-01T23:00:00Z"
    },
    "temperature": {
        "type": "Property",
        "value": 12.803,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "CEL"
    },
    "relativeHumidity": {
        "type": "Property",
        "value": 0.9082,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "C62"
    },
    "atmosphericPressure": {
        "type": "Property",
        "value": 1034.656,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "A97"
    },
    "windDirection": {
        "type": "Property",
        "value": 269,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "DD"
    },
    "windSpeed": {
        "type": "Property",
        "value": 4.28,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "MTS"
    },
    "gustSpeed": {
        "type": "Property",
        "value": 5.51,
        "observedAt": "2024-06-01T23:00:00Z",
        "unitCode": "MTS"
    }
}
```

`observedAt` sits beside each value as the temporal property that records when the reading was observed (ETSI GS CIM 009 v1.9.1, clause 4.8), and `unitCode` beside it gives the reading's unit (clause 4.5.2). M2's 12.182 knots of wind become 6.27 metres per second, and its 85.059 percent humidity becomes the ratio 0.8506.
