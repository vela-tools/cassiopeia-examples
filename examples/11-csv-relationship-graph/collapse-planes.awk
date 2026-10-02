# Collapses the OpenFlights aircraft list to one row per IATA code.
#
# A route names its aircraft by IATA code, and the plane mapping keys each AircraftModel on that code
# so the route's hasAircraftModel resolves. But an IATA code can cover several of the list's rows: CN1
# is four Cessna types, E7W both Embraer 175 wings. Mapped as they are, those rows share one entity id
# and only one of them survives, chosen by whichever is processed first. Collapsing them first leaves
# one row per id, whose name lists every model the code covers, joined by "; ": the names themselves
# already use "/" ("Hawker/Raytheon 700/800"), but none uses a semicolon. The ICAO code is kept when
# the rows agree on one and left out when they do not, since no one code then describes every merged
# model. A row with no IATA code is keyed on its ICAO code and passes through unchanged.
#
# Output columns are the input's: name, IATA code, ICAO code, in order of first appearance.
#
# Usage: awk -f openflights-csv.awk -f collapse-planes.awk planes.dat > aircraft-models.csv

{
    parse_row($0, fields)
    name = fields[1]
    iata = fields[2]
    icao = fields[3]

    # A row without an IATA code gets a key of its own, prefixed with SUBSEP so it can never equal
    # an IATA code, and is therefore never merged with another row.
    key = (iata == "") ? SUBSEP NR : iata
    if (!(key in names)) {
        order[++rows] = key
        names[key] = name
        codes_iata[key] = iata
        codes_icao[key] = icao
        next
    }
    names[key] = names[key] "; " name
    if (codes_icao[key] != icao) {
        codes_icao[key] = ""
    }
}

END {
    for (row = 1; row <= rows; row++) {
        key = order[row]
        print format_field(names[key]) "," format_field(codes_iata[key]) "," format_field(codes_icao[key])
    }
}
