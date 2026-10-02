# Reads and writes OpenFlights rows, for the scripts that collapse them.
#
# OpenFlights quotes every text field, escapes a quote inside one by doubling it, and writes a missing
# value as an unquoted \N. A quoted field may hold a comma ("Bonaire, Saint Eustatius and Saba"), so
# splitting on FS would cut it in two; these functions walk the quotes instead. They use only POSIX
# awk, so the preparation step does not depend on which awk the system ships.

# Splits one row into fields[1..n] and returns n. \N becomes the empty string. A trailing carriage
# return is dropped, so a copy of the file saved with CRLF line endings reads the same.
function parse_row(line, fields,    count, position, length_of_line, character, value, quoted) {
    sub(/\r$/, "", line)
    # The caller passes the same array for every row; emptying it first keeps a short row from
    # inheriting the trailing fields of a longer one.
    split("", fields)
    count = 0
    position = 1
    length_of_line = length(line)
    while (position <= length_of_line + 1) {
        value = ""
        quoted = 0
        if (substr(line, position, 1) == "\"") {
            quoted = 1
            position++
            while (position <= length_of_line) {
                character = substr(line, position, 1)
                if (character == "\"" && substr(line, position + 1, 1) == "\"") {
                    value = value "\""
                    position += 2
                } else if (character == "\"") {
                    position++
                    break
                } else {
                    value = value character
                    position++
                }
            }
        }
        while (position <= length_of_line && substr(line, position, 1) != ",") {
            value = value substr(line, position, 1)
            position++
        }
        if (!quoted && value == "\\N") {
            value = ""
        }
        fields[++count] = value
        position++
    }
    return count
}

# Writes one field the way OpenFlights does: quoted text, or \N for a missing value.
function format_field(value) {
    if (value == "") {
        return "\\N"
    }
    gsub(/"/, "\"\"", value)
    return "\"" value "\""
}
