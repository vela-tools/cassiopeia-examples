# Running an example

Every example runs the same way: download its dataset, run its Cassiopeia commands, look at what came out.

## What you need

Cassiopeia, as either a native binary or a container image.

For the binary, download the archive for your platform from the [latest release](https://github.com/vela-tools/cassiopeia/releases/latest), unzip it, and put `cassiopeia` on your `PATH`. Check it with `cassiopeia --version`.

The other option is Docker or Podman with no Cassiopeia installed at all. The image is `ghcr.io/vela-tools/cassiopeia:latest`. ecCodes is already inside it, so GRIB1 works as well as GRIB2 with nothing added to the host.

You also need `curl` for the datasets. A handful of examples want `awk`, `jq`, `zip`, or `docker compose`; each of those pages says so, and continuous integration installs the same set. A Rust toolchain is only for the runner below. No example needs one.

## From the page

Each example page carries every command in full, in both forms. The page is all you need:

```bash
cd examples/01-json-field-mapping
mkdir -p data
curl --fail --location \
    --output data/element.json \
    https://raw.githubusercontent.com/andrejewski/periodic-table/master/data.json
cassiopeia map \
    --input data/element.json \
    --mapping element.json5 \
    --type json \
    --output out \
    --context none
```

The container form mounts the example directory at `/data` and makes it the working directory. Relative paths mean the same thing either way:

```bash
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "$PWD:/data" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:latest \
    map \
    --input data/element.json \
    --mapping element.json5 \
    --type json \
    --output out \
    --context none
```

For Podman, replace `docker` with `podman` and drop the `--user` line. Rootless Podman already maps the container's root to your user, and forcing an identity there writes files nobody on the host owns. Docker does need the line. Without it, `out/` comes back owned by `root`.

## With the runner

`tools/runner` does the same three steps, then checks the output against what the page claims. A Rust toolchain is the price, which is why it stays optional:

```bash
cargo run -- run 01
cargo run -- run 01 --runtime docker
cargo run -- run 01 --runtime podman
```

Name an example by its ordinal, its directory name, or a path: `01`, `01-json-field-mapping`, and `examples/01-json-field-mapping` all reach the same one. `--skip-fetch` keeps the datasets already on disk. `--skip-verify` leaves the output unchecked. `--image` pins a release in place of `latest`. `--drift-report drift.json` writes what a live source has drifted to as JSON.

A run can end three ways. It passes; it fails, and the example is broken; or it passes with drift, and a source the descriptor calls `live` has moved under a mapping that still works. Drift leaves the exit code alone.

The `volatility` on each `[[datasets]]` block decides which of the three you get. Against a `frozen` source every difference is a defect. Against a `live` one a difference is drift, until the count falls below half the declared figure or climbs past twice it. Outside those rails the mapping is broken, no matter what the publisher has done.

In container mode the runner builds the command shown above, with your real uid and gid in place of the shell expansion. Add `:Z` to the volume argument when you run it by hand on a host with SELinux enforcing.

Examples that need the Smart Data Models catalog mount it from `${XDG_CACHE_HOME:-$HOME/.cache}/cassiopeia-examples/schemas`, so one example's download is still there for the next. Without that mount, `--rm` would take the catalog away with the container.

[Running with Docker](https://vela-tools.github.io/cassiopeia/docs/reference/docker) covers the image, its paths, and its environment variables in full.

## The datasets

No dataset is stored in this repository. Each example's `example.toml` records the URL, the publisher, the licence, the record count, and whether the publisher has finished with the file; the runner downloads from there into the example's `data/` directory. Run an example twice and it downloads twice. Publishers change files in place, and a stale copy stops testing anything. `--skip-fetch` keeps what you already have.

The runner tries again when a publisher is under load. A refused connection, a 429, or a 5xx buys another attempt after a widening wait. A 404 does not: that is a move, and waiting will not undo it. However the file arrives, the runner then checks that it exists, holds something, and parses as JSON when the name says it should. A publisher's holding page therefore fails at the download, well before some later stage tries to map it.

Publishers also move files for good. If a fetch reports a 404, or the mapping suddenly finds no records, open an [issue](https://github.com/vela-tools/cassiopeia-examples/issues/new/choose) instead of working around it locally.

## Examples that need more

Validating against a published Smart Data Model takes the schema catalog. Those examples declare `smart-data-models = true` and run `cassiopeia sdm download` first, and every later example reuses what that leaves behind.

The broker examples declare a `[context-broker]` block naming a `docker-compose.yml` and the URL the broker answers at. The runner brings that stack up, waits for the broker to serve the API, and takes it down again at the end, whether the mapping succeeded or failed. Their pages walk through the same thing by hand. One extra flag shows up in the container form, `--network host`, because the URL in the manifest is `localhost`.

A scheduled example never exits on its own. By hand you stop it with an interrupt, and the runner does the same, reading `[schedule]` for how many cycles are worth watching and interrupting once they have had time to happen.

All of them run in continuous integration, and no descriptor can opt out.

## Cleaning up

```bash
rm -rf examples/*/data examples/*/out
```

Outside the example directory, container runs leave only the schema catalog under `~/.cache/cassiopeia-examples`, and native runs the usual Cassiopeia directories.
