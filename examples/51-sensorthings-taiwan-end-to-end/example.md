# Keeping a broker up to date from Taiwan's Civil IoT services

Each earlier example showed one SensorThings concept on the server that shows it best. This one combines them on one platform. Taiwan's Civil IoT services publish air quality, weather, rain, storm sewer and flood data as separate SensorThings servers under one host. One manifest reads five of them for the Taipei area, each incrementally through its own cursor, and maps each to its Smart Data Model. Every fifteen minutes it delivers what all five gained into a local broker's temporal API, as one atomic cycle. On the way it handles problems a real deployment runs into: reading names that are not identifiers, sentinel values, a water level that needs its station's datum, a filter that goes through a Thing's properties, a place read from a FeatureOfInterest, and services that no longer answer.

## Get the data

Taiwan's [Civil IoT](https://ci.taiwan.gov.tw/dsp/) platform gathered the country's government sensor networks into SensorThings services on FROST-Server, version 1.0. The project ended on 2025-12-31, and the services moved from `sta.ci.taiwan.gov.tw` to `https://sta.colife.org.tw/<service>/v1.0`, where the main ones are still served. The datasets are published under Taiwan's [Open Government Data License, version 1.0](https://data.gov.tw/license), as the platform's listing on the [national dataset catalogue](https://ark.nchc.org.tw/en/organization/civiliot) states. A request takes about a second from Europe.

Each of the five inputs selects Datastreams with a filter the server applies:

| Input | Service | Selection | Data streams | Cadence |
| --- | --- | --- | --- | --- |
| Air | `STA_AirQuality_EPAIoT` | `Thing/properties/city eq '臺北市' and name eq 'PM2.5'` | 101 | about a minute |
| Weather | `STA_Weather` | Taipei City, eight properties | 152 | hourly |
| Rain | `STA_Rain` | `Thing/properties/CountyName eq '臺北市' and name eq 'Past10Min'` | 72 | ten minutes |
| Storm sewers | `STA_RainSewer` | `Thing/properties/CountyName eq '臺北市'` | 128 | ten minutes |
| Flood depth | `STA_WaterResource_v2` | `ObservedProperty/name eq '淹水深度' and substringof('新北市',Thing/properties/authority)` | 128 | five minutes to an hour |

List what each selection offers before mapping it. The weather stations, for example:

```bash
cassiopeia sensorthings readings https://sta.colife.org.tw/STA_Weather/v1.0 \
    --datastreams "Thing/properties/CountyName eq '臺北市'"
```

```text
reading                             unit     unit code  data streams  multi data streams  things with repeats
AirPressure                         hPa      A97        19            0                   0
AirTemperature                      °C       CEL        19            0                   0
DailyExtremeHighAirTemperatureTime  hhmm     NA         19            0                   0
PeakGustSpeed                       m/sec    MTS        19            0                   0
Precipitation                       mm       MMT        19            0                   0
RelativeHumidity                    %        P1         19            0                   0
SunshineDuration                    hh       NA         9             0                   0
UVIndex                             UVI      NA         9             0                   0
Weather                             NA       NA         19            0                   0
WindDirection                       degrees  unknown    19            0                   0
WindSpeed                           m/sec    MTS        19            0                   0
```

The rows are cut to the ones this page discusses. Taipei City has 19 weather stations. Only the nine staffed ones serve `UVIndex`, `SunshineDuration` and visibility. Read the `unit code` column with care here. `degrees` resolves to nothing. The service writes `NA` where a reading has no unit, and `NA` resolves to `NA`, a real UN/CEFACT code, so `cefact_unit` would give the `Weather` text a unit. The weather mapping therefore writes every `unitCode` as a constant. The `...Time` readings serve full timestamps as text under a unit of `hhmm`. The mapping leaves them out, along with the other properties the selection does not name.

The other selections list one property each: `PM2.5` in `μg/m3` (`GQ`), `Past10Min` in `mm`, `水位` (water level) in `m`, and `淹水深度` (flood depth) in `cm`.

## Get the schema

```bash
cassiopeia sdm download
```

## Start the broker

The [docker-compose.yaml](docker-compose.yaml) is the stack of [example 46](../46-sensorthings-incremental-broker/example.md). It starts [Andromeda](https://andromeda.skvarc.com) and its PostgreSQL server, with the broker on port 1056:

```bash
docker compose up -d
```

## Selecting by a Thing's properties

The services' Datastreams carry no place. Their Things do, in `properties`, and a `$filter` can follow the navigation from a Datastream to its Thing: `Thing/properties/CountyName eq '臺北市'` selects the Datastreams of the Things whose county is Taipei City. Each service names these properties its own way, `city` in one and `CountyName` in another. A service's Things show which name it uses.

The flood depth service has neither. Its Things carry the authority that runs them, and Taipei City runs none of this service's flood sensors. The input therefore reads those of New Taipei City, which surrounds Taipei, with `substringof('新北市',Thing/properties/authority)`. The property filter, `ObservedProperty/name eq '淹水深度'`, matters as much. The same service serves camera frames as Datastreams, with image URLs stamped `2099-12-31T00:00:00Z`. An incremental read would never read them, because it waits for readings stamped before the cycle starts. A latest read would write the year 2099 as an `observedAt`.

## Five cursors

```json5
read: { incremental: { cursor: "taiwan-air", initial: "1h", lookback: "30m" } },
```

Each input names its own cursor: `taiwan-air`, `taiwan-weather`, `taiwan-rain`, `taiwan-sewer` and `taiwan-flood`. A cursor records the server it reads, and a manifest in which two inputs name the same cursor is refused when it loads. `initial` follows each service's cadence: an hour for the services that report every few minutes, three hours for the hourly weather stations and for the flood sensors, which report once an hour while the street is dry. [Example 46](../46-sensorthings-incremental-broker/example.md) explains the positions, the lookback, and how to inspect and reset a cursor.

All five inputs deliver into one output with `atomicity: "atomic"`, so a cycle is one unit. If one service still fails after the schedule's retries, the cycle delivers nothing from any of them and moves none of the five cursors. The next cycle then reads all of it again.

## The air quality sensors

```json5
location: {
    type: "GeoProperty",
    source: "{{ readings['PM2.5'].featureOfInterest.feature }}",
},
```

The input sets `featuresOfInterest: true`, so each reading carries the FeatureOfInterest of its Observation: the place where it was taken (OGC 18-088, section 8.2.8). For a fixed sensor that is the sensor's own point, and the record's `location` would give the same point if the server's Locations were right. Of the 101 sensors selected as Taipei, 19 serve their Location with latitude and longitude swapped: `[22.673094, 120.488045]` is a point in southern Taiwan written the wrong way round. None of those 19 reported in the hour before this was written, so an incremental read gave them no records. The FeatureOfInterest of every reading that did arrive was correct. `featuresOfInterest` is meant for a Thing that moves, such as a sensor on a bus, because it places each reading where it was taken. `location` here has no `observedAt`, so series output keeps it once, from the sensor's first reading of the cycle.

The reading's key, `PM2.5`, holds a dot, so every template reads it with brackets: `readings['PM2.5']`. The same goes for `水位` and `淹水深度`, which Tera does not read unquoted as identifiers. Each record of an incremental read holds one Observation, and each of these inputs selects one property. Every record therefore has its reading, and the brackets need no guard.

Each Datastream's Sensor is served too, as `readings['PM2.5'].sensor`. Its `name` is the platform's URN for the sensor, such as `urn:moenv:station:aqi:10639632197`. Its `encodingType` says SensorML, but its `metadata` is a text dump. Smart Data Models describe a sensor as a `Device`, which an observation links through `refDevice`. This example leaves the Sensors out.

## Sentinel values

```json5
temperature: {
    source: "{% if readings.AirTemperature %}{% if readings.AirTemperature.result > -90 %}{{ readings.AirTemperature.result }}{% endif %}{% endif %}",
    transformation: "float",
    properties: {
        unitCode: {
            source: "CEL",
        },
        observedAt: {
            source: "{{ readings.AirTemperature.time }}",
            transformation: "datetime",
        },
    },
},
```

The weather service marks a missing value with `-99`, and a missing precipitation with `-990`, instead of leaving the Observation out. In a single hour, eight of the 19 stations served `-99` as their gust speed, six as their air pressure, and one served `-990.0` as its precipitation. Written as they are, these would become readings. The models set a minimum of 0 on pressure, speed and precipitation, so validation would fail and stop the run. Each guard drops such a value: a temperature below -90, or anything else below 0. The outer `if` leaves the attribute out of a record of another property, since a comparison fails on a missing field. `relativeHumidity` is divided by 100, from percent to the fraction the model takes. `weatherType` keeps the `Weather` reading's text, such as `陰` (overcast).

Rain gauges are separate Things from the weather stations, so they become separate entities: `WeatherObserved` with only `precipitation`, from `Past10Min`. Each gauge serves nine accumulation windows, from `Now` and `Past10Min` up to `Past3days`. The ten-minute window matches the gauges' cadence, so each reading is the rain of its own ten minutes and no rain is counted twice.

## A water level above its datum

```json5
waterLevel: {
    source: "{% set level = readings['水位'].result - readings['水位'].datastream.description | split(pat='Datastream_Base_Elev=') | last | float %}{% if level >= 0 %}{{ level | round(precision=3) }}{% endif %}",
    transformation: "float",
    properties: {
        unitCode: {
            source: "MTR",
        },
        observedAt: {
            source: "{{ readings['水位'].time }}",
            transformation: "datetime",
        },
    },
},
```

The storm sewer service serves the water surface as an elevation, which is negative where a sewer lies below the datum. The monitoring point 百齡12 read `-0.79` m. `WaterObserved` gives `waterLevel` a minimum of 0, and an elevation is not what the model means. The service writes each point's base elevation as text into its Datastream's description:

```text
stationID=256df54d-5bf7-fcfd-e394-8a6e6bb18bdb, stationName=百齡12, Datastream_id=00a675b6-808c-7152-fbd0-a494f51c9c23, Datastream_name=水位, Datastream_Description=雨水下水道監測水位, Datastream_Base_Elev=-1.4
```

The template takes the number after `Datastream_Base_Elev=` and writes the height of the surface above that base: the depth of water at the point, here 0.61 m. When this was written, every point that reported served a base below its surface. The difference is rounded, because subtracting these decimals in binary floating point gives `0.6099999999999999`.

Flood depth is simpler. It becomes `FloodMonitoring`'s `currentLevel`, served in centimetres and written in metres, with the sensor's station code as `stationID`. Most streets read `0` most of the time.

## Services the example leaves out

The platform's other services answer, but not usefully:

- `STA_AirQuality_v2`, the national air quality stations, holds no Observations; [example 48](../48-sensorthings-multiple-locations/example.md) maps its Things and Locations.
- `STA_WaterResource_v3` holds no Observations, and its `@iot.nextLink` points to the old host, `sta.ci.taiwan.gov.tw`. Cassiopeia refuses a link to a host other than the service root's, because the input's headers would be sent there too. A read of this service therefore fails at its second page.
- `STA_Earthquake_v2` answers every collection with `500 Internal Server Error`.

## The request budget

In each cycle, an incremental read sends one listing per service, then one Observations request per data stream: about 580 requests here. The five inputs are read side by side, so a cycle took about a minute when this was written, well inside the fifteen-minute interval. `delay` spaces the requests of each input by 100 milliseconds. A wider selection, such as all of Taiwan's EPAIoT sensors, is 33,000 Datastreams, and a cycle of it would not end before the next one is due.

## Run it

```bash
cassiopeia map \
    --manifest manifest.json5
```

The same run in a container mounts this directory at `/data` and makes it the working directory, so the paths do not change. For Podman, replace `docker` with `podman` and drop the `--user` line: rootless Podman already maps the container's root to your user. `--network host` lets the container reach the broker at `localhost:1056`. The cursors live in the container's `/var/lib/cassiopeia/cursors`. To resume across containers, add a volume for that directory, as [example 46](../46-sensorthings-incremental-broker/example.md) explains.

```bash
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --network host \
    --volume "$PWD:/data" \
    --volume "$HOME/.cache/cassiopeia-examples/schemas:/var/lib/cassiopeia/schemas" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.2.0 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner starts the broker, runs two cycles fifteen minutes apart, counts the broker's entities of each `dataModel.Environment` type, checks one entity of each service at the temporal endpoint, and takes the broker down:

```bash
cargo run -- run 51
cargo run -- run 51 --runtime docker
```

The first cycle fires at once. When this was written, it read 7,699 Observations into 406 entities: 62 air quality sensors, 19 weather stations, 70 rain gauges, 127 sewer monitoring points and 128 flood sensors. Stop the run with Ctrl-C.

## Read the result

Ask the temporal API for one entity of each service, with the context of its model. The air quality sensor:

```bash
curl "http://localhost:1056/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:AirQualityObserved:taiwan-air-10639632197" \
    -H 'Link: <https://raw.githubusercontent.com/smart-data-models/dataModel.Environment/master/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json"'
```

After the first cycle, Andromeda held 37 PM2.5 readings of the sensor, a minute apart, cut here to the first two:

```json
{
    "id": "urn:ngsi-ld:AirQualityObserved:taiwan-air-10639632197",
    "type": "AirQualityObserved",
    "pm25": [
        {
            "type": "Property",
            "value": 5.94,
            "unitCode": "GQ",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee79-2b3f-7704-a6d1-df433987ef68",
            "observedAt": "2026-11-18T10:45:33Z"
        },
        {
            "type": "Property",
            "value": 5.94,
            "unitCode": "GQ",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee79-2b3f-7704-a6d1-df44b4270596",
            "observedAt": "2026-11-18T10:46:33Z"
        }
    ]
}
```

The temporal representation holds only the attributes with an `observedAt`. `name`, `location` and `dateObserved` describe the sensor. Andromeda keeps them with the entity's current state, which `/ngsi-ld/v1/entities/` returns, as [example 46](../46-sensorthings-incremental-broker/example.md) shows.

The weather station 天母, Tianmu, in the context of `dataModel.Weather`:

```bash
curl "http://localhost:1056/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:WeatherObserved:taiwan-weather-C0A9C0" \
    -H 'Link: <https://raw.githubusercontent.com/smart-data-models/dataModel.Weather/master/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json"'
```

After the first cycle, the station held three hours of readings. `temperature`, `weatherType` and `gustSpeed` are cut to their first two instances, and the other series (`relativeHumidity`, `windSpeed`, `windDirection`, `atmosphericPressure` and `precipitation`) are left out:

```json
{
    "id": "urn:ngsi-ld:WeatherObserved:taiwan-weather-C0A9C0",
    "type": "WeatherObserved",
    "gustSpeed": [
        {
            "type": "Property",
            "value": 8.6,
            "unitCode": "MTS",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee79-2cd6-75d9-842d-e5764cebf766",
            "observedAt": "2026-11-18T09:00:00Z"
        }
    ],
    "temperature": [
        {
            "type": "Property",
            "value": 22.4,
            "unitCode": "CEL",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee79-2cd6-75d9-842d-e5835dee5833",
            "observedAt": "2026-11-18T09:00:00Z"
        },
        {
            "type": "Property",
            "value": 22.1,
            "unitCode": "CEL",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee79-2cd6-75d9-842d-e5840a20a20d",
            "observedAt": "2026-11-18T10:00:00Z"
        }
    ],
    "weatherType": [
        {
            "type": "Property",
            "value": "陰",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee79-2cd6-75d9-842d-e5707d4f895c",
            "observedAt": "2026-11-18T09:00:00Z"
        },
        {
            "type": "Property",
            "value": "陰",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee79-2cd6-75d9-842d-e571ab03549e",
            "observedAt": "2026-11-18T10:00:00Z"
        }
    ]
}
```

`gustSpeed` has one instance where the other series have three: in two of the three hours the station served `-99`, which the mapping dropped.

The rain gauge `urn:ngsi-ld:WeatherObserved:taiwan-rain-01A410`, the sewer monitoring point `urn:ngsi-ld:WaterObserved:taiwan-sewer-256df54d-5bf7-fcfd-e394-8a6e6bb18bdb` and the flood sensor `urn:ngsi-ld:FloodMonitoring:taiwan-flood-3822500FL0112` answer the same query under their own ids. Each attribute is a series of instances, each with an `observedAt` and an `instanceId` the broker assigned (ETSI GS CIM 009 v1.9.1, clauses 4.5.5 and 4.5.7). Each cycle adds the readings its services gained.

When you are done, stop the broker and discard its data, and reset the five cursors if you want the next run to start over:

```bash
docker compose down -v
cassiopeia sensorthings cursor reset taiwan-air
cassiopeia sensorthings cursor reset taiwan-weather
cassiopeia sensorthings cursor reset taiwan-rain
cassiopeia sensorthings cursor reset taiwan-sewer
cassiopeia sensorthings cursor reset taiwan-flood
```
