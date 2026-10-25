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
use url::Url;

use crate::error::NyaError;
use crate::types::Repository;
use std::path::Path;

fn format_snippet(snippet: &str) -> String {
    if snippet.is_empty() {
        String::new()
    } else {
        format!(", cerca de {snippet:?}")
    }
}

fn format_reason(reason: &Option<&'static str>) -> String {
    match reason {
        Some(field) => format!(", falta el campo {field}"),
        None => String::new(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(
        "error de parseo en línea {line}, columna {column}{}{}",
        format_reason(.reason),
        format_snippet(.snippet)
    )]
    Invalid {
        line: usize,
        column: usize,
        snippet: String,
        reason: Option<&'static str>,
    },
    #[error(
        "contenido no consumido tras el último bloque en línea {line}, columna {column}: {rest:?}"
    )]
    Unconsumed {
        line: usize,
        column: usize,
        rest: String,
    },
}

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

fn skip_preamble(input: &str) -> IResult<&str, ()> {
    let mut input = input;
    loop {
        if let Ok((rest, _)) = skip_comment(input) {
            input = rest;
            continue;
        }
        if let Ok((rest, _)) = blank_line(input) {
            if rest.len() == input.len() {
                break;
            }
            input = rest;
            continue;
        }
        break;
    }
    Ok((input, ()))
}

fn is_supported_uri(uri: &str) -> bool {
    match Url::parse(uri) {
        Ok(parsed) => matches!(parsed.scheme(), "http" | "https"),
        Err(_) => false,
    }
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

type BlockEntry<'a> = Result<Repository, (&'static str, &'a str)>;

fn parse_block(input: &str) -> IResult<&str, BlockEntry<'_>> {
    let (mut input, _) = skip_preamble(input)?;
    let block_start = input;
    let mut uris = Vec::new();
    let mut suites = Vec::new();
    let mut components = Vec::new();
    let mut signed_by = String::new();

    while let Ok((next_input, (key, value))) = parse_key_value_line(input) {
        match key.as_str() {
            "URIs" => {
                for uri in value.split_whitespace() {
                    if !is_supported_uri(uri) {
                        return Err(nom::Err::Error(nom::error::Error::new(
                            input,
                            nom::error::ErrorKind::Verify,
                        )));
                    }
                    uris.push(uri.to_string());
                }
            }
            "Suites" => {
                suites.extend(value.split_whitespace().map(String::from));
            }
            "Components" => {
                components.extend(value.split_whitespace().map(String::from));
            }
            "Signed-By" => {
                signed_by = value;
            }
            _ => (), // Ignore unknown keys
        }

        let (next_input, _) = clean_inside_block(next_input)?;
        input = next_input;
    }

    let parsed_any_key =
        !uris.is_empty() || !suites.is_empty() || !components.is_empty() || !signed_by.is_empty();

    match Repository::new(uris, suites, components, signed_by) {
        Ok(repo) => Ok((input, Ok(repo))),
        Err(_) if !parsed_any_key => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        ))),
        Err(field) => Ok((input, Err((field, block_start)))),
    }
}

fn block_separator(input: &str) -> IResult<&str, ()> {
    value((), many1(blank_line)).parse(input)
}

fn parse_file(input: &str) -> IResult<&str, Vec<BlockEntry<'_>>> {
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

fn parse_error(content: &str, rest: &str, reason: Option<&'static str>) -> ParseError {
    let (line, column) = error_position(content, rest.trim_start());
    let snippet = line_snippet(content, rest.trim_start());
    ParseError::Invalid {
        line,
        column,
        snippet,
        reason,
    }
}

pub(crate) fn parse_content(content: &str) -> Result<Vec<Repository>, ParseError> {
    let has_content = content.lines().any(|line| {
        let line = line.trim_start();
        !line.is_empty() && !line.starts_with('#')
    });
    if !has_content {
        return Ok(Vec::new());
    }

    let (rest, blocks) = parse_file(content).map_err(|e| {
        let rest = match &e {
            nom::Err::Error(err) | nom::Err::Failure(err) => err.input,
            nom::Err::Incomplete(_) => "",
        };
        parse_error(content, rest, None)
    })?;

    let repositories = blocks
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|(field, at)| parse_error(content, at, Some(field)))?;

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
        return Err(ParseError::Unconsumed {
            line,
            column,
            rest: rest.to_string(),
        });
    }

    Ok(repositories)
}

pub(crate) async fn read_repositories_from_file(
    filepath: impl AsRef<Path>,
) -> Result<Vec<Repository>, NyaError> {
    let filepath = filepath.as_ref();
    let file_content = fs::read_to_string(filepath)
        .await
        .map_err(|source| NyaError::io("leer", filepath, source))?;
    parse_content(&file_content).map_err(|source| NyaError::Parse {
        path: filepath.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_line_after_last_block_returns_err() {
        let content = "URIs: http://example.com\nSuites: stable\nComponents: main\nSigned-By: /tmp/test-keyring.gpg\n\nmalformed garbage line without colon";
        let err = parse_content(content).unwrap_err();
        assert!(
            matches!(
                &err,
                ParseError::Unconsumed {
                    line: 6,
                    column: 1,
                    ..
                }
            ),
            "una línea malformada tras el último bloque debe producir Unconsumed en 6:1: {err:?}"
        );
    }

    #[test]
    fn trailing_comment_without_newline_returns_ok() {
        let content = "URIs: http://example.com\nSuites: stable\nComponents: main\nSigned-By: /tmp/test-keyring.gpg\n\n# comentario final sin newline";
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
    fn missing_field_error_reports_block_start() {
        let content = "URIs: http://example.com\nSuites: stable\n\nmalformed garbage";
        let err = parse_content(content).unwrap_err();
        match &err {
            ParseError::Invalid {
                line,
                column,
                reason,
                ..
            } => {
                assert_eq!((*line, *column), (1, 1), "posición del error: {err:?}");
                assert_eq!(
                    *reason,
                    Some("Components"),
                    "el motivo debe nombrar el campo que falta: {err:?}"
                );
            }
            other => panic!("esperaba ParseError::Invalid: {other}"),
        }
    }

    #[test]
    fn missing_field_error_names_the_field() {
        let content = "URIs: http://a.example.com\nSuites: stable\nComponents: main\nSigned-By: /tmp/k.gpg\n\nURIs: http://b.example.com\nSuites: stable\n";
        let err = parse_content(content).unwrap_err();
        match &err {
            ParseError::Invalid {
                line,
                column,
                reason,
                ..
            } => {
                assert_eq!(
                    *reason,
                    Some("Components"),
                    "el error debe nombrar el campo: {err:?}"
                );
                assert_eq!(
                    (*line, *column),
                    (6, 1),
                    "apunta al inicio del bloque defectuoso: {err:?}"
                );
            }
            other => panic!("esperaba ParseError::Invalid: {other}"),
        }
        assert!(
            err.to_string().contains("falta el campo Components"),
            "el mensaje debe incluir el campo: {err}"
        );
    }

    #[test]
    fn parses_sources_fixture_with_content_assertions() {
        let repos = parse_content(include_str!("fixtures/sources")).unwrap();
        assert_eq!(repos.len(), 2, "la fixture debe producir 2 repositorios");

        assert_eq!(
            repos[0].uris().to_vec(),
            vec![
                "http://mirror1.example.com/ubuntu",
                "http://mirror2.example.com/ubuntu"
            ],
            "el primer repo debe acumular sus 2 URIs y ignorar claves desconocidas"
        );
        assert_eq!(repos[0].suites().to_vec(), vec!["noble", "noble-updates"]);
        assert_eq!(repos[0].components().to_vec(), vec!["main", "universe"]);
        assert_eq!(
            repos[0].signed_by(),
            "/usr/share/keyrings/ubuntu-archive-keyring.gpg"
        );

        assert_eq!(
            repos[1].uris().to_vec(),
            vec!["http://archive.example.com/debian"]
        );
        assert_eq!(repos[1].suites().to_vec(), vec!["stable"]);
        assert_eq!(repos[1].components().to_vec(), vec!["main"]);
        assert_eq!(
            repos[1].signed_by(),
            "/usr/share/keyrings/debian-archive-keyring.gpg"
        );
    }

    #[test]
    fn missing_required_field_returns_err() {
        let content = "URIs: http://example.com\nSuites: stable\n";
        let err = parse_content(content).unwrap_err();
        assert!(
            matches!(&err, ParseError::Invalid { .. }),
            "un bloque sin Components debe producir ParseError::Invalid: {err:?}"
        );
    }

    #[test]
    fn missing_signed_by_returns_err() {
        let content = "URIs: http://example.com\nSuites: stable\nComponents: main\n";
        let err = parse_content(content).unwrap_err();
        assert!(
            matches!(&err, ParseError::Invalid { .. }),
            "un bloque sin Signed-By debe fallar en is_valid: {err:?}"
        );
    }

    #[test]
    fn non_http_scheme_is_rejected() {
        for uri in [
            "file:///etc/apt",
            "ftp://example.com/repo",
            "mirror1.example.com",
        ] {
            let content =
                format!("URIs: {uri}\nSuites: stable\nComponents: main\nSigned-By: /tmp/k.gpg\n");
            let err = parse_content(&content).unwrap_err();
            match &err {
                ParseError::Invalid {
                    line,
                    column,
                    reason,
                    ..
                } => {
                    assert_eq!(
                        (*line, *column),
                        (1, 1),
                        "la posición debe apuntar a la línea del URIs: {err:?}"
                    );
                    assert_eq!(
                        *reason, None,
                        "un fallo de sintaxis nom no lleva motivo de campo: {err:?}"
                    );
                }
                other => panic!("esperaba ParseError::Invalid: {other}"),
            }
        }
    }

    #[test]
    fn https_uri_is_accepted() {
        let content = "URIs: https://example.com/ubuntu\nSuites: stable\nComponents: main\nSigned-By: /tmp/k.gpg\n";
        let repos = parse_content(content).unwrap();
        assert_eq!(repos.len(), 1, "https debe aceptarse igual que http");
        assert_eq!(repos[0].uris().to_vec(), vec!["https://example.com/ubuntu"]);
    }

    #[test]
    fn crlf_content_parses_like_lf() {
        let lf = include_str!("fixtures/sources");
        let crlf = lf.replace('\n', "\r\n");

        let repos_lf = parse_content(lf).unwrap();
        let repos_crlf = parse_content(&crlf).unwrap();

        assert_eq!(repos_lf.len(), repos_crlf.len());
        for (a, b) in repos_lf.iter().zip(repos_crlf.iter()) {
            assert_eq!(a, b, "CRLF y LF deben producir repositorios idénticos");
        }
    }

    #[test]
    fn folded_uris_continuation_is_silently_dropped() {
        // Documenta el comportamiento actual: una continuación indentada
        // de deb822 (folding) no se entiende y se descarta como clave
        // desconocida — la segunda URI se pierde sin error.
        let content = "URIs: http://mirror1.example.com/ubuntu\n http://mirror2.example.com/ubuntu\nSuites: stable\nComponents: main\nSigned-By: /tmp/test-keyring.gpg\n";
        let repos = parse_content(content).unwrap();
        assert_eq!(
            repos[0].uris().to_vec(),
            vec!["http://mirror1.example.com/ubuntu"],
            "la URI plegada en la línea de continuación se pierde (limitación actual)"
        );
    }

    #[tokio::test]
    async fn read_repositories_missing_file_names_the_path() {
        let err = read_repositories_from_file("/no/existe/sources")
            .await
            .unwrap_err();
        assert!(
            matches!(&err, NyaError::Io { path, .. } if path == "/no/existe/sources"),
            "el error debe portar el path del fichero: {err:?}"
        );
    }

    #[tokio::test]
    async fn read_repositories_invalid_content_reports_filename() {
        let path = std::env::temp_dir().join(format!("nya-parser-test-{}", std::process::id()));
        std::fs::write(
            &path,
            "URIs: http://example.com\nSuites: stable\nComponents: main\nSigned-By: /tmp/k.gpg\n\ngarbage sin dos puntos\n",
        )
        .unwrap();

        let err = read_repositories_from_file(path.to_str().unwrap())
            .await
            .unwrap_err();
        std::fs::remove_file(&path).ok();

        match &err {
            NyaError::Parse {
                path: reported,
                source,
            } => {
                assert!(
                    reported.contains("nya-parser-test"),
                    "el error debe nombrar el fichero: {reported}"
                );
                assert!(
                    matches!(
                        source,
                        ParseError::Unconsumed {
                            line: 6,
                            column: 1,
                            ..
                        }
                    ),
                    "posición del contenido sin consumir: {source:?}"
                );
            }
            other => panic!("esperaba NyaError::Parse: {other:?}"),
        }
    }
}
