# Collapses the OpenFlights country list to one row per country name.
#
# Airports and airlines name their country as text, and the country mapping keys each Country on that
# name so their inCountry links resolve. But the list has one row per DAFIF code, not per country:
# Palestine appears as GZ and as WE, India as IN and as BS. Mapped as they are, those rows share one
# entity id and only one DAFIF code survives, chosen by whichever row is processed first. Collapsing
# them first leaves one row per id that carries every code, separated by spaces, for the mapping to
# split into a list. A row without a DAFIF code contributes none.
#
# Output columns are the input's: name, ISO code, DAFIF codes, in order of first appearance. Rows of one
# country are expected to agree on its ISO code; should a later list disagree, keeping either code
# would be a silent guess, so the step stops instead.
#
# Usage: awk -f openflights-csv.awk -f collapse-countries.awk countries.dat > countries.csv

{
    parse_row($0, fields)
    name = fields[1]
    iso = fields[2]
    dafif = fields[3]

    if (!(name in codes_iso)) {
        order[++rows] = name
        codes_iso[name] = iso
        codes_dafif[name] = dafif
        next
    }
    if (iso != codes_iso[name]) {
        print "collapse-countries: " name " has ISO codes " codes_iso[name] " and " iso > "/dev/stderr"
        failed = 1
        exit 1
    }
    if (dafif != "") {
        codes_dafif[name] = (codes_dafif[name] == "") ? dafif : codes_dafif[name] " " dafif
    }
}

END {
    if (failed) {
        exit 1
    }
    for (row = 1; row <= rows; row++) {
        name = order[row]
        print format_field(name) "," format_field(codes_iso[name]) "," format_field(codes_dafif[name])
    }
}
