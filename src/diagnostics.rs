use crate::context::*;
use crate::position::*;
use crate::types::*;
use crate::util::*;
use itertools::Itertools;
use jsonrpc_core::Params;
use lsp_types::*;
use std::path::Path;

pub fn publish_diagnostics(params: Params, ctx: &mut Context) {
    let params: PublishDiagnosticsParams = params.parse().expect("Failed to parse params");
    let session = ctx.session.clone();
    let client = None;
    let path = params.uri.to_file_path().unwrap();
    let buffile = path.to_str().unwrap();
    ctx.diagnostics
        .insert(buffile.to_string(), params.diagnostics);
    let document = ctx.documents.get(buffile);
    if document.is_none() {
        return;
    }
    let document = document.unwrap();
    let version = document.version;
    let diagnostics = &ctx.diagnostics[buffile];
    let ranges = diagnostics
        .iter()
        .map(|x| {
            format!(
                "{}|{}",
                lsp_range_to_kakoune(&x.range, &document.text, &ctx.offset_encoding),
                match x.severity {
                    Some(DiagnosticSeverity::Error) => "DiagnosticError",
                    _ => "DiagnosticWarning",
                }
            )
        })
        .join(" ");

    let mut error_count = 0;
    let mut warning_count = 0;
    let line_flags = diagnostics
        .iter()
        .map(|x| {
            format!(
                "{}|{}",
                x.range.start.line + 1,
                match x.severity {
                    Some(DiagnosticSeverity::Error) => {
                        error_count += 1;
                        "%opt[lsp_diagnostic_line_error_sign]"
                    }
                    _ => {
                        warning_count += 1;
                        "%opt[lsp_diagnostic_line_warning_sign]"
                    }
                }
            )
        })
        .join(" ");
    // Always show a space on line one if no other highlighter is there,
    // to make sure the column always has the right width
    // Also wrap it in another eval and quotes, to make sure the %opt[] tags are expanded
    let command = format!(
        "set buffer lsp_diagnostic_error_count {}
         set buffer lsp_diagnostic_warning_count {}
         set buffer lsp_errors {} {}
         eval \"set buffer lsp_error_lines {} {} '0| '\"",
        error_count, warning_count, version, ranges, version, line_flags
    );
    let command = format!(
        "
        eval -buffer {} {}",
        editor_quote(buffile),
        editor_quote(&command)
    );
    let meta = EditorMeta {
        session,
        client,
        buffile: buffile.to_string(),
        filetype: "".to_string(), // filetype is not used by ctx.exec, but it's definitely a code smell
        version,
        fifo: None,
    };
    ctx.exec(meta, command.to_string());
}

pub fn editor_diagnostics(meta: EditorMeta, ctx: &mut Context) {
    let content = ctx
        .diagnostics
        .iter()
        .flat_map(|(filename, diagnostics)| {
            diagnostics
                .iter()
                .map(|x| {
                    let p = get_kakoune_position(filename, &x.range.start, ctx).unwrap();
                    format!(
                        "{}:{}:{}: {}:{}",
                        filename,
                        p.line,
                        p.column,
                        match x.severity {
                            Some(DiagnosticSeverity::Error) => "error",
                            _ => "warning",
                        },
                        x.message
                    )
                })
                .collect::<Vec<_>>()
        })
        .join("\n");
    let command = format!(
        "lsp-show-diagnostics {} {}",
        editor_quote(&ctx.root_path),
        editor_quote(&content),
    );
    ctx.exec(meta, command);
}
