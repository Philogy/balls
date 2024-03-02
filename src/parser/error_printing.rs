use crate::parser::{tokens::Token, types::Span};
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::error::{Simple, SimpleReason};

pub fn print_errors<'a, 'b, F>(
    src: &'a str,
    file_path: &'b str,
    errs: Vec<Simple<Token>>,
    mut token_span_resolver: F,
) -> bool
where
    F: FnMut(&Span) -> Span,
{
    let errored = !errs.is_empty();

    errs.into_iter()
        .map(|err| err.map(|tok| tok.to_string()))
        .for_each(|err| {
            let src_span = token_span_resolver(&err.span());

            let report = Report::build(ReportKind::Error, &file_path, src_span.start);

            let main_err_label = (&file_path, src_span);

            let report = match err.reason() {
                SimpleReason::Unclosed { span, delimiter } => report
                    .with_message(format!(
                        "Unclosed delimiter {}",
                        delimiter.fg(Color::Yellow)
                    ))
                    .with_label(
                        Label::new((&file_path, token_span_resolver(&span)))
                            .with_message(format!(
                                "Unclosed delimiter {}",
                                delimiter.fg(Color::Yellow)
                            ))
                            .with_color(Color::Yellow),
                    )
                    .with_label(
                        Label::new(main_err_label)
                            .with_message(format!(
                                "Must be closed before this {}",
                                err.found()
                                    .unwrap_or(&"end of file".to_string())
                                    .fg(Color::Red)
                            ))
                            .with_color(Color::Red),
                    ),
                SimpleReason::Unexpected => report
                    .with_message(format!(
                        "{} (parser expecting: [{}])",
                        match err.found() {
                            Some(_) => "Unexpected token in input",
                            None => "Unexpected EOF",
                        },
                        err.expected()
                            .map(|expected| match expected {
                                Some(s) => s,
                                None => "<EOF>",
                            })
                            .collect::<Vec<_>>()
                            .join(",")
                    ))
                    .with_label(
                        Label::new(main_err_label)
                            .with_message(format!(
                                "Unexpected token {}",
                                err.found()
                                    .unwrap_or(&"end of file".to_string())
                                    .fg(Color::Red)
                            ))
                            .with_color(Color::Red),
                    ),

                SimpleReason::Custom(msg) => report.with_message(msg).with_label(
                    Label::new(main_err_label)
                        .with_message(format!("{}", msg.fg(Color::Red)))
                        .with_color(Color::Red),
                ),
            };

            report
                .finish()
                .print((&file_path, Source::from(&src)))
                .expect("failed to print error report");
        });

    errored
}
