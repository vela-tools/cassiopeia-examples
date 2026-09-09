use crate::command_line_token::{CommandLineToken, TokenRole};

/// The separator that continues a command onto the next line, as every example page writes it.
const CONTINUATION: &str = " \\\n    ";

/// Renders lines as the canonical multi-line shell form every example page uses.
///
/// Page text and executed command come from the same tokens, so a page cannot drift away from the
/// run it documents.
pub fn render(lines: &[String]) -> String {
    lines.join(CONTINUATION)
}

/// Groups tokens into one line per flag, keeping a flag and its value together.
///
/// A flag opens a line, and the token after it joins that line unless it opens one of its own. A
/// token that is neither, such as a subcommand, stands alone.
pub fn group_arguments(tokens: &[CommandLineToken]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut pending: Option<&CommandLineToken> = None;

    for token in tokens {
        match (token.role(), pending.take()) {
            (TokenRole::Flag, Some(flag)) => {
                lines.push(flag.as_str().to_owned());
                pending = Some(token);
            }
            (TokenRole::Flag, None) => pending = Some(token),
            (TokenRole::Value, Some(flag)) => lines.push(format!("{flag} {token}")),
            (TokenRole::Value, None) => lines.push(token.as_str().to_owned()),
        }
    }

    if let Some(flag) = pending {
        lines.push(flag.as_str().to_owned());
    }

    lines
}

#[cfg(test)]
mod tests {
    use crate::{
        command_line::{group_arguments, render},
        command_line_token::CommandLineToken,
    };

    fn tokens(values: &[&str]) -> Vec<CommandLineToken> {
        values.iter().filter_map(|value| CommandLineToken::try_from((*value).to_owned()).ok()).collect()
    }

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn a_flag_and_its_value_share_one_line() {
        let grouped = group_arguments(&tokens(&["--input", "data/element.json", "--output", "out"]));

        assert_eq!(grouped, lines(&["--input data/element.json", "--output out"]));
    }

    #[test]
    fn a_subcommand_stands_on_its_own_line() {
        let grouped = group_arguments(&tokens(&["map", "--type", "json"]));

        assert_eq!(grouped, lines(&["map", "--type json"]));
    }

    #[test]
    fn a_flag_without_a_value_keeps_its_own_line() {
        let grouped = group_arguments(&tokens(&["--input", "x", "--dry-run"]));

        assert_eq!(grouped, lines(&["--input x", "--dry-run"]));
    }

    #[test]
    fn rendering_continues_every_line_but_the_last() {
        let rendered = render(&lines(&["cassiopeia map", "--output out"]));

        assert_eq!(rendered, "cassiopeia map \\\n    --output out");
    }

    #[test]
    fn a_single_line_is_rendered_without_a_continuation() {
        assert_eq!(render(&lines(&["cassiopeia sdm download"])), "cassiopeia sdm download");
    }

    #[test]
    fn no_tokens_render_to_nothing() {
        assert_eq!(render(&group_arguments(&tokens(&[]))), String::new());
    }
}
