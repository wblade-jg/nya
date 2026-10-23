use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{char, line_ending, multispace0, space0},
    combinator::{eof, map, opt, value},
    multi::{many1, separated_list1},
    sequence::{delimited, preceded, separated_pair, terminated},
};
use tokio::fs;
use tokio::io;

use crate::types::Repository;

fn skip_comment(input: &str) -> IResult<&str, ()> {
    value(
        (),
        preceded(
            char('#'),
            terminated(opt(is_not("\r\n")), alt((line_ending, eof))),
        ),
    )
    .parse(input)
}

fn blank_line(input: &str) -> IResult<&str, ()> {
    value((), terminated(space0, alt((line_ending, eof)))).parse(input)
}

fn clean_inside_block(input: &str) -> IResult<&str, ()> {
    let mut input = input;
    loop {
        if let Ok((rest, _)) = skip_comment(input) {
            input = rest;
            continue;
        }
        if blank_line(input).is_ok() {
            break;
        }
        let (rest, _) = space0(input)?;
        input = rest;
        break;
    }
    Ok((input, ()))
}

fn parse_key_value_line(input: &str) -> IResult<&str, (String, String)> {
    terminated(
        map(
            separated_pair(is_not(":\r\n"), tag(":"), opt(is_not("\r\n"))),
            |(k, v): (&str, Option<&str>)| {
                (k.trim().to_string(), v.unwrap_or("").trim().to_string())
            },
        ),
        alt((line_ending, eof)),
    )
    .parse(input)
}

fn parse_block(input: &str) -> IResult<&str, Repository> {
    let (mut input, _) = clean_inside_block(input)?;
    let mut repo = Repository::new();

    while let Ok((next_input, (key, value))) = parse_key_value_line(input) {
        match key.as_str() {
            "URIs" => {
                repo.uris.extend(value.split_whitespace().map(String::from));
            }
            "Suites" => {
                repo.suites
                    .extend(value.split_whitespace().map(String::from));
            }
            "Components" => {
                repo.components
                    .extend(value.split_whitespace().map(String::from));
            }
            "Signed-By" => {
                repo.signed_by = value;
            }
            _ => (), // Ignore unknown keys
        }

        let (next_input, _) = clean_inside_block(next_input)?;
        input = next_input;
    }

    if !repo.is_valid() {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )))
    } else {
        Ok((input, repo))
    }
}

fn block_separator(input: &str) -> IResult<&str, ()> {
    value((), many1(blank_line)).parse(input)
}

fn parse_file(input: &str) -> IResult<&str, Vec<Repository>> {
    delimited(
        multispace0,
        separated_list1(block_separator, parse_block),
        multispace0,
    )
    .parse(input)
}

fn error_position(content: &str, rest: &str) -> (usize, usize) {
    let offset = content.len().saturating_sub(rest.len());
    let before = &content[..offset];
    let line = before.matches('\n').count() + 1;
    let column = offset - before.rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
    (line, column)
}

fn line_snippet(content: &str, rest: &str) -> String {
    let (line, _) = error_position(content, rest);
    let text = content.lines().nth(line - 1).unwrap_or("").trim();
    text.chars().take(60).collect()
}

fn parse_error(content: &str, rest: &str) -> io::Error {
    let (line, column) = error_position(content, rest.trim_start());
    let snippet = line_snippet(content, rest.trim_start());
    let base = format!("error de parseo en línea {line}, columna {column}");
    let message = if snippet.is_empty() {
        base
    } else {
        format!("{base}, cerca de {snippet:?}")
    };
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(crate) fn parse_content(content: &str) -> io::Result<Vec<Repository>> {
    let hay_contenido = content.lines().any(|line| {
        let line = line.trim_start();
        !line.is_empty() && !line.starts_with('#')
    });
    if !hay_contenido {
        return Ok(Vec::new());
    }

    let (rest, repositories) = parse_file(content).map_err(|e| {
        let rest = match &e {
            nom::Err::Error(err) | nom::Err::Failure(err) => err.input,
            nom::Err::Incomplete(_) => "",
        };
        parse_error(content, rest)
    })?;

    let mut rest = rest;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        match skip_comment(rest) {
            Ok((next, _)) => rest = next,
            Err(_) => break,
        }
    }

    if !rest.is_empty() {
        let (line, column) = error_position(content, rest);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "contenido no consumido tras el último bloque en línea {line}, columna {column}: {rest:?}"
            ),
        ));
    }

    Ok(repositories)
}

pub(crate) async fn read_repositories_from_file(filepath: &str) -> io::Result<Vec<Repository>> {
    let file_content = fs::read_to_string(filepath).await.map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("No se pudo leer el fichero {filepath}: {e}"),
        )
    })?;
    parse_content(&file_content).map_err(|e| io::Error::new(e.kind(), format!("{filepath}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_line_after_last_block_returns_err() {
        let content = "URIs: http://example.com\nSuites: stable\nComponents: main\n\nmalformed garbage line without colon";
        let result = parse_content(content);
        assert!(
            result.is_err(),
            "una línea malformada tras el último bloque debe producir Err"
        );
    }

    #[test]
    fn trailing_comment_without_newline_returns_ok() {
        let content = "URIs: http://example.com\nSuites: stable\nComponents: main\n\n# comentario final sin newline";
        let result = parse_content(content);
        assert!(
            result.is_ok(),
            "un fichero que termina en comentario sin newline final debe parsear OK"
        );
        let repos = result.unwrap();
        assert_eq!(repos.len(), 1);
    }

    #[test]
    fn empty_or_comment_only_content_returns_no_repositories() {
        for content in ["", "   \n\n", "# solo un comentario\n", "\n # indented\n\n"] {
            let repos = parse_content(content)
                .unwrap_or_else(|e| panic!("contenido {content:?} debe parsear sin error: {e}"));
            assert!(
                repos.is_empty(),
                "contenido {content:?} no debe producir repositorios"
            );
        }
    }

    #[test]
    fn parse_error_reports_line_and_column() {
        let content = "URIs: http://example.com\nSuites: stable\n\nmalformed garbage";
        let err = parse_content(content).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("línea 4") && message.contains("columna 1"),
            "el error debe indicar línea y columna: {message}"
        );
    }
}
