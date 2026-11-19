# Reading new SensorThings readings into a broker on a schedule

[Example 44](../44-sensorthings-range-read/example.md) read a fixed span before each run, so a run every hour over six hours reads every reading six times. This example reads each reading once. An incremental read keeps a cursor, a small store that records how far it has read each data stream. Each cycle reads only what the server gained since the last successful cycle. A schedule runs a cycle every five minutes, and each cycle delivers its readings to the temporal API of a local NGSI-LD broker in one piece.

## Get the data

The source is the river gauge network of [example 43](../43-sensorthings-latest-read/example.md): the FROST-Server of the Thuringian State Office for Environment, Mining and Nature Conservation (TLUBN) at `https://kshww2.thueringen.de/FROST-Server/v1.1`. The server names TLUBN as the owner of the data and states no licence for the service. The selection is the same 15-minute water level and discharge, and the mapping is [example 44](../44-sensorthings-range-read/example.md)'s, unchanged:

```bash
cassiopeia sensorthings readings https://kshww2.thueringen.de/FROST-Server/v1.1 \
    --datastreams "properties/type eq 'TH_PEGELDATEN_15MIN'"
```

## Get the schema

```bash
cassiopeia sdm download
```

## Start the broker

The [docker-compose.yaml](docker-compose.yaml) starts [Andromeda](https://andromeda.skvarc.com), an NGSI-LD broker, and the PostgreSQL server that holds its data, with PostGIS and TimescaleDB. Andromeda serves the NGSI-LD API on port 1056, including the temporal endpoints. The database publishes no port, so only the broker can reach it, and the file can give it a fixed password. Start it and wait for it to become healthy:

```bash
docker compose up -d
```

## The incremental read

```json5
sensorthings: {
    datastreams: "properties/type eq 'TH_PEGELDATEN_15MIN'",
    read: {
        incremental: {
            cursor: "thueringen-gauges",
            initial: "2h",
            lookback: "1h",
        },
    },
    delay: "100ms",
},
```

- `cursor` names the cursor, a store called `thueringen-gauges.redb` in Cassiopeia's cursors folder. It keeps one position per data stream. A position holds the high-water mark, which is the time of the latest reading written for that data stream, and the Observations written within the lookback behind the mark. The cursor also records the server it was read from, and refuses an input that reads another server through it.
- `initial` is where a data stream without a position starts: two hours before the cycle. The first cycle therefore reads two hours of every gauge, as a range read of two hours would.
- `lookback` is how far behind its high-water mark each data stream is read again. A reading that a gauge uploads late, stamped before the mark, is still read, and only once, because a reading the position already holds is dropped. SensorThings gives no entity an insertion time, so a reading uploaded later than the lookback looks the same as one already read. It is never read. Choose the lookback from the server's usual upload delay. This server uploads within minutes.

Each cycle lists the selected Datastreams, as a range read does. It then reads the Observations of each one up to the instant the cycle started, from a lookback behind its mark, or from `initial` when it has no position. The data streams furthest behind go first. A reading stamped at or after the cycle start waits for a later cycle. That keeps an incremental read away from the reading stamped in the year 2251, which [example 43](../43-sensorthings-latest-read/example.md)'s latest read wrote as it was.

An incremental read gives one record per Observation, as a range read does, so the output is `temporal.representation: "series"`.

## A cursor moves only after delivery

```json5
output: {
    target: "context-broker",
    url: "http://localhost:1056/",
    operation: "temporal",
    atomicity: "atomic",
    temporal: {
        representation: "series",
    },
    context: "default",
    validation: {
        mode: "fail-when-schema",
    },
},
```

`operation: "temporal"` sends each entity to the broker's temporal API, which keeps every instance as history (ETSI GS CIM 009 v1.9.1, clause 5.6.11). `atomicity: "atomic"` holds the cycle back until every record has been read, mapped and validated, then delivers all of it. A cursor moves only once the whole cycle has succeeded, and all of its positions move in one write. A cycle that fails at the server, in validation or at the broker therefore sends nothing and moves nothing. The next cycle reads those readings again and sends them once. With `atomicity: "streaming"`, a failed cycle may already have sent part of its readings, and the next cycle sends them again.

## The schedule

```json5
schedule: {
    mode: "every",
    value: "5m",
    jitter: "20s",
    retry: {
        maxAttempts: 3,
        backoff: "30s",
    },
},
onFailure: "continue",
```

A cycle runs every five minutes. The gauges report every quarter of an hour at staggered times, so most cycles find something new. A cycle that finds nothing sends nothing. `retry` repeats a failed cycle up to three times, 30 seconds apart. Every attempt starts from the same instant and the same positions, so a retried attempt reads the same readings. `onFailure: "continue"` keeps the schedule running past a cycle whose retries are spent, and makes the command exit non-zero when it ends. [Bound and harden the schedule](https://vela-tools.github.io/cassiopeia/docs/guides/scheduling#bound-and-harden-the-schedule) covers both.

## Run it

```bash
cassiopeia map \
    --manifest manifest.json5
```

The same run in a container mounts this directory at `/data` and makes it the working directory, so the paths do not change. For Podman, replace `docker` with `podman` and drop the `--user` line: rootless Podman already maps the container's root to your user. `--network host` lets the container reach the broker at `localhost:1056`.

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

From the repository root, the runner starts the broker, runs two cycles, stops the run, checks the broker, and takes the broker down:

```bash
cargo run -- run 46
cargo run -- run 46 --runtime docker
```

The first cycle fires at once. When this was written, it read the last two hours, 1,307 readings of 95 gauges, in about 25 seconds. The second cycle, five minutes later, read only the readings uploaded since. Stop the run with Ctrl-C.

## Inspect the cursor

The cursor outlives the run. Once the run has stopped, list the cursors and show one:

```bash
cassiopeia sensorthings cursor list
cassiopeia sensorthings cursor show thueringen-gauges
```

```text
cursor             service root                                    data streams  oldest high water     newest high water
thueringen-gauges  https://kshww2.thueringen.de/FROST-Server/v1.1  190           2026-11-18T10:00:00Z  2026-11-18T11:00:00Z
```

```text
service root: https://kshww2.thueringen.de/FROST-Server/v1.1
kind        id    high water            seen  last listed           missing since
datastream  7430  2026-11-18T10:45:00Z  5     2026-11-18T11:08:03Z  -
datastream  7431  2026-11-18T10:45:00Z  5     2026-11-18T11:08:03Z  -
datastream  7433  2026-11-18T10:45:00Z  5     2026-11-18T11:08:03Z  -
```

`show` prints one row per position. It gives the data stream's kind and id, its high-water mark, and how many Observations it has seen within the lookback. The last two columns are the start of the last cycle that listed it and the start of the first cycle that stopped listing it, or `-` while it is still listed. 190 of the 250 Datastreams have a position. The others had no reading in the first two hours and get one when they report. A Datastream that drops out of the selection keeps its position and resumes from it when it is listed again. Until then, a `collect-datastreams-missing` warning names it.

The next run of the manifest resumes from these positions, however long ago the last one ran. To start over, remove them, or set them back to read part of the history again, for instance after fixing a mapping:

```bash
cassiopeia sensorthings cursor reset thueringen-gauges
cassiopeia sensorthings cursor reset thueringen-gauges --to 3h
cassiopeia sensorthings cursor reset thueringen-gauges --datastream 7668 --to 3h
```

Without `--to`, a reset removes the positions, so each data stream starts at `initial` again. With `--to`, an RFC 3339 instant or a span before now, it sets the high-water marks there, and the next cycle reads each data stream from that point minus the lookback and sends those readings again. `--datastream` limits a reset to one Datastream.

A run holds the stores of its cursors from start to end. A second run through the same cursor is refused with `run-cursor-unusable` before it sends a request. A cursor command against a running job's cursor is refused too: it fails with `cursor 'thueringen-gauges' is in use by a running job` and changes nothing. `cursor list` reads every cursor in the folder, so it fails this way while any job runs.

## Where the cursors live

Cursors live in the folder that the `cursors.folder` setting names, `CASSIOPEIA_CURSORS__FOLDER` in the environment. It defaults to `cursors` under Cassiopeia's data directory. In the container that is `/var/lib/cassiopeia/cursors`, and `--rm` removes the container with the folder when it stops. One run keeps its cursor across all of its cycles, and that is what the runner checks. To resume where a container left off, keep the folder on a volume by adding `--volume "$PWD/cursors:/var/lib/cassiopeia/cursors"` to the command above. [Persisting cursors](https://vela-tools.github.io/cassiopeia/docs/reference/docker#persisting-cursors) covers it.

## Read the result

Ask the broker's temporal API for one gauge. The query carries the model's context, so the broker writes the attribute names short:

```bash
curl "http://localhost:1056/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:WaterObserved:thueringen-570280" \
    -H 'Link: <https://raw.githubusercontent.com/smart-data-models/dataModel.Environment/master/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json"'
```

After two cycles, Andromeda held seven water levels and seven discharges of Rothenstein, a quarter of an hour apart, from 09:45 to 11:15. All of them came from the first cycle's two hours: the gauge had uploaded nothing newer when the second cycle ran, so that cycle sent nothing for it. Cut to the first two instances of each attribute:

```json
{
    "id": "urn:ngsi-ld:WaterObserved:thueringen-570280",
    "type": "WaterObserved",
    "flow": [
        {
            "type": "Property",
            "value": 18.4,
            "unitCode": "MQS",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee71-3560-768e-b80b-6be127e32927",
            "observedAt": "2026-11-18T09:45:00Z"
        },
        {
            "type": "Property",
            "value": 18.9,
            "unitCode": "MQS",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee71-3560-768e-b80b-6be2700df0f1",
            "observedAt": "2026-11-18T10:00:00Z"
        }
    ],
    "waterLevel": [
        {
            "type": "Property",
            "value": 1.01,
            "unitCode": "MTR",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee71-3560-768e-b80b-6be8858efed4",
            "observedAt": "2026-11-18T09:45:00Z"
        },
        {
            "type": "Property",
            "value": 1.02,
            "unitCode": "MTR",
            "instanceId": "urn:ngsi-ld:attribute:instance:01a1ee71-3560-768e-b80b-6be95e2f5af8",
            "observedAt": "2026-11-18T10:00:00Z"
        }
    ]
}
```

The broker gives each instance an `instanceId` of its own (ETSI GS CIM 009 v1.9.1, clause 4.5.7). No reading appears twice: a cycle reads each gauge from an hour behind its mark and drops what an earlier cycle wrote. The temporal representation holds the attributes that have an `observedAt`, the ones that change with each reading. The others, `name`, `description`, `location` and `dateObserved`, describe the gauge. Andromeda keeps them in its current state, which the entities endpoint returns:

```bash
curl "http://localhost:1056/ngsi-ld/v1/entities/urn:ngsi-ld:WaterObserved:thueringen-570280" \
    -H 'Link: <https://raw.githubusercontent.com/smart-data-models/dataModel.Environment/master/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json"'
```

```json
{
    "id": "urn:ngsi-ld:WaterObserved:thueringen-570280",
    "type": "WaterObserved",
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-18T09:45:00Z"
    },
    "description": {
        "type": "Property",
        "value": "Gauge on the Saale"
    },
    "flow": {
        "type": "Property",
        "value": 19.7,
        "unitCode": "MQS",
        "observedAt": "2026-11-18T11:15:00Z"
    },
    "name": {
        "type": "Property",
        "value": "Rothenstein"
    },
    "waterLevel": {
        "type": "Property",
        "value": 1.04,
        "unitCode": "MTR",
        "observedAt": "2026-11-18T11:15:00Z"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                11.605473698714054,
                50.847739111118464
            ]
        }
    }
}
```

`waterLevel` and `flow` there are the latest instances of their series. The same query by type, `?type=WaterObserved`, counts every gauge the cycles wrote, 100 after the second one.

When you are done, stop the broker and discard its data, and reset the cursor if you want the next run to start over:

```bash
docker compose down -v
cassiopeia sensorthings cursor reset thueringen-gauges
```
