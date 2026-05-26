use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use lzvm_artifacts::expression_info::HintInfo;
use lzvm_artifacts::hint_program::SOURCE_LOOKUP_ASSUMES_HINT;
use lzvm_pil::{
    lex_source, parse_expression_tokens, FunctionStatement, LexError, SourceFile, Token, TokenKind,
};

use crate::source_statement_hints::{
    lower_structured_source_lookup_line, source_lookup_line_hint, source_statement_line,
    SourceLookupInputs,
};
use crate::source_static_values::{evaluate_source_static_expression, static_value_integer};

const U8_RANGE_CHECK_OPID: u64 = 100;
const U16_RANGE_CHECK_OPID: u64 = 101;
const FIRST_SPECIFIED_RANGE_CHECK_OPID: u64 = U16_RANGE_CHECK_OPID;
const U8_MAX: i128 = 0xFF;
const U16_MAX: i128 = 0xFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SourceRangeCheckKey {
    min: i128,
    max: i128,
    predefined: bool,
    absorb: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SourceRangeCheckIds {
    last_assigned_opid: u64,
    opids: BTreeMap<SourceRangeCheckKey, u64>,
}

impl Default for SourceRangeCheckIds {
    fn default() -> Self {
        Self {
            last_assigned_opid: FIRST_SPECIFIED_RANGE_CHECK_OPID,
            opids: BTreeMap::new(),
        }
    }
}

impl SourceRangeCheckIds {
    fn opid(&mut self, min: i128, max: i128, predefined: bool) -> Option<u64> {
        if min >= max {
            return None;
        }
        let absorb = predefined && min >= 0 && max <= U16_MAX;
        for (key, opid) in &self.opids {
            if key.min == min
                && key.max == max
                && (key.predefined == predefined || key.absorb == absorb)
            {
                return Some(*opid);
            }
        }

        let opid = self.generate_opid(min, max, predefined)?;
        self.opids.insert(
            SourceRangeCheckKey {
                min,
                max,
                predefined,
                absorb,
            },
            opid,
        );
        Some(opid)
    }

    fn generate_opid(&mut self, min: i128, max: i128, predefined: bool) -> Option<u64> {
        if predefined && min >= 0 {
            if max <= U8_MAX {
                return Some(U8_RANGE_CHECK_OPID);
            }
            if max <= U16_MAX {
                return Some(U16_RANGE_CHECK_OPID);
            }
        }
        self.last_assigned_opid = self.last_assigned_opid.checked_add(1)?;
        Some(self.last_assigned_opid)
    }
}

pub(crate) fn lower_source_range_check_statement(
    inputs: &SourceLookupInputs<'_>,
    range_checks: &RefCell<SourceRangeCheckIds>,
    statement: &FunctionStatement,
) -> Result<Vec<HintInfo>, LexError> {
    let line = source_statement_line(inputs.module, statement);
    let tokens = lex_source(&line)?;
    if source_range_dual_byte_call(&tokens) {
        return lower_source_range_dual_byte_statement(inputs, &line, &tokens);
    }
    if source_multi_range_check_call(&tokens) {
        return lower_source_multi_range_check_statement(inputs, range_checks, &line, &tokens);
    }
    if !source_range_check_call(&tokens) {
        return Ok(Vec::new());
    }
    let Some(open_index) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
    else {
        return Ok(Vec::new());
    };
    let Some(close_index) = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::RParen)
    else {
        return Ok(Vec::new());
    };
    let Some(arguments) = top_level_argument_ranges(&tokens, open_index, close_index) else {
        return Ok(Vec::new());
    };
    let Some(call) = source_range_check_arguments(&tokens, arguments) else {
        return Ok(Vec::new());
    };
    let Some(expression) = source_argument_text(&line, &tokens, call.expression) else {
        return Ok(Vec::new());
    };
    let selector = call
        .selector
        .and_then(|range| source_argument_text(&line, &tokens, range))
        .unwrap_or_else(|| "1".to_owned());
    let Some(min) = source_static_integer_argument(inputs, &line, &tokens, call.min) else {
        return Ok(Vec::new());
    };
    let Some(max) = source_static_integer_argument(inputs, &line, &tokens, call.max) else {
        return Ok(Vec::new());
    };
    let predefined = match call.predefined {
        Some(range) => {
            let Some(value) = source_static_integer_argument(inputs, &line, &tokens, range) else {
                return Ok(Vec::new());
            };
            value != 0
        }
        None => false,
    };
    if predefined && min >= 0 && max <= U16_MAX && !(min == 0 && (max == U8_MAX || max == U16_MAX))
    {
        return Ok(Vec::new());
    }
    let Some(opid) = range_checks.borrow_mut().opid(min, max, predefined) else {
        return Ok(Vec::new());
    };

    let lookup_line = format!("lookup_assumes({opid}, [{expression}], sel: {selector})");
    Ok(vec![lower_source_lookup_assumes_line(inputs, lookup_line)?])
}

fn lower_source_range_dual_byte_statement(
    inputs: &SourceLookupInputs<'_>,
    line: &str,
    tokens: &[Token],
) -> Result<Vec<HintInfo>, LexError> {
    let Some(open_index) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
    else {
        return Ok(Vec::new());
    };
    let Some(close_index) = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::RParen)
    else {
        return Ok(Vec::new());
    };
    let Some(arguments) = top_level_argument_ranges(tokens, open_index, close_index) else {
        return Ok(Vec::new());
    };
    let Some(call) = source_range_dual_byte_arguments(tokens, arguments) else {
        return Ok(Vec::new());
    };
    let Some(byte_a) = source_argument_text(line, tokens, call.byte_a) else {
        return Ok(Vec::new());
    };
    let Some(byte_b) = source_argument_text(line, tokens, call.byte_b) else {
        return Ok(Vec::new());
    };
    let selector = call
        .selector
        .and_then(|range| source_argument_text(line, tokens, range))
        .unwrap_or_else(|| "1".to_owned());
    let lookup_line =
        format!("lookup_assumes(DUAL_RANGE_BYTE_ID, [{byte_a}, {byte_b}], sel: {selector})");
    Ok(vec![lower_source_lookup_assumes_line(inputs, lookup_line)?])
}

fn lower_source_multi_range_check_statement(
    inputs: &SourceLookupInputs<'_>,
    range_checks: &RefCell<SourceRangeCheckIds>,
    line: &str,
    tokens: &[Token],
) -> Result<Vec<HintInfo>, LexError> {
    let Some(open_index) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
    else {
        return Ok(Vec::new());
    };
    let Some(close_index) = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::RParen)
    else {
        return Ok(Vec::new());
    };
    let Some(arguments) = top_level_argument_ranges(tokens, open_index, close_index) else {
        return Ok(Vec::new());
    };
    let Some(call) = source_multi_range_check_arguments(tokens, arguments) else {
        return Ok(Vec::new());
    };
    let Some(expression) = source_argument_text(line, tokens, call.expression) else {
        return Ok(Vec::new());
    };
    let range_selector = call
        .range_selector
        .and_then(|range| source_argument_text(line, tokens, range))
        .unwrap_or_else(|| "1".to_owned());
    let selector = call
        .selector
        .and_then(|range| source_argument_text(line, tokens, range))
        .unwrap_or_else(|| "1".to_owned());
    let predefined = match call.predefined {
        Some(range) => {
            let Some(value) = source_static_integer_argument(inputs, line, tokens, range) else {
                return Ok(Vec::new());
            };
            value != 0
        }
        None => false,
    };
    let Some(min1) = source_static_integer_argument(inputs, line, tokens, call.min1) else {
        return Ok(Vec::new());
    };
    let Some(max1) = source_static_integer_argument(inputs, line, tokens, call.max1) else {
        return Ok(Vec::new());
    };
    let Some(min2) = source_static_integer_argument(inputs, line, tokens, call.min2) else {
        return Ok(Vec::new());
    };
    let Some(max2) = source_static_integer_argument(inputs, line, tokens, call.max2) else {
        return Ok(Vec::new());
    };
    if predefined || (min1 == min2 && max1 == max2) {
        return Ok(Vec::new());
    }
    let Some(opid1) = range_checks.borrow_mut().opid(min1, max1, predefined) else {
        return Ok(Vec::new());
    };
    let Some(opid2) = range_checks.borrow_mut().opid(min2, max2, predefined) else {
        return Ok(Vec::new());
    };
    let first_selector = format!("({selector}) * ({range_selector})");
    let second_selector = format!("({selector}) * (1 - ({range_selector}))");
    let first_line = format!("lookup_assumes({opid1}, [{expression}], sel: {first_selector})");
    let second_line = format!("lookup_assumes({opid2}, [{expression}], sel: {second_selector})");
    Ok(vec![
        lower_source_lookup_assumes_line(inputs, first_line)?,
        lower_source_lookup_assumes_line(inputs, second_line)?,
    ])
}

fn lower_source_lookup_assumes_line(
    inputs: &SourceLookupInputs<'_>,
    lookup_line: String,
) -> Result<HintInfo, LexError> {
    Ok(
        lower_structured_source_lookup_line(inputs, SOURCE_LOOKUP_ASSUMES_HINT, &lookup_line)?
            .unwrap_or_else(|| source_lookup_line_hint(SOURCE_LOOKUP_ASSUMES_HINT, lookup_line)),
    )
}

#[derive(Debug, Clone, Copy)]
struct SourceRangeCheckArguments {
    expression: (usize, usize),
    min: (usize, usize),
    max: (usize, usize),
    selector: Option<(usize, usize)>,
    predefined: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, Default)]
struct PartialSourceRangeCheckArguments {
    expression: Option<(usize, usize)>,
    min: Option<(usize, usize)>,
    max: Option<(usize, usize)>,
    selector: Option<(usize, usize)>,
    predefined: Option<(usize, usize)>,
}

impl PartialSourceRangeCheckArguments {
    fn finish(self) -> Option<SourceRangeCheckArguments> {
        Some(SourceRangeCheckArguments {
            expression: self.expression?,
            min: self.min?,
            max: self.max?,
            selector: self.selector,
            predefined: self.predefined,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceMultiRangeCheckArguments {
    expression: (usize, usize),
    min1: (usize, usize),
    max1: (usize, usize),
    min2: (usize, usize),
    max2: (usize, usize),
    range_selector: Option<(usize, usize)>,
    selector: Option<(usize, usize)>,
    predefined: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, Default)]
struct PartialSourceMultiRangeCheckArguments {
    expression: Option<(usize, usize)>,
    min1: Option<(usize, usize)>,
    max1: Option<(usize, usize)>,
    min2: Option<(usize, usize)>,
    max2: Option<(usize, usize)>,
    range_selector: Option<(usize, usize)>,
    selector: Option<(usize, usize)>,
    predefined: Option<(usize, usize)>,
}

impl PartialSourceMultiRangeCheckArguments {
    fn finish(self) -> Option<SourceMultiRangeCheckArguments> {
        Some(SourceMultiRangeCheckArguments {
            expression: self.expression?,
            min1: self.min1?,
            max1: self.max1?,
            min2: self.min2?,
            max2: self.max2?,
            range_selector: self.range_selector,
            selector: self.selector,
            predefined: self.predefined,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceRangeDualByteArguments {
    byte_a: (usize, usize),
    byte_b: (usize, usize),
    selector: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, Default)]
struct PartialSourceRangeDualByteArguments {
    byte_a: Option<(usize, usize)>,
    byte_b: Option<(usize, usize)>,
    selector: Option<(usize, usize)>,
}

impl PartialSourceRangeDualByteArguments {
    fn finish(self) -> Option<SourceRangeDualByteArguments> {
        Some(SourceRangeDualByteArguments {
            byte_a: self.byte_a?,
            byte_b: self.byte_b?,
            selector: self.selector,
        })
    }
}

fn source_range_check_call(tokens: &[Token]) -> bool {
    let Some(name) = tokens.first() else {
        return false;
    };
    let Some(open) = tokens.get(1) else {
        return false;
    };
    name.kind == TokenKind::Identifier
        && name.lexeme == "range_check"
        && open.kind == TokenKind::LParen
}

fn source_multi_range_check_call(tokens: &[Token]) -> bool {
    let Some(name) = tokens.first() else {
        return false;
    };
    let Some(open) = tokens.get(1) else {
        return false;
    };
    name.kind == TokenKind::Identifier
        && name.lexeme == "multi_range_check"
        && open.kind == TokenKind::LParen
}

fn source_range_dual_byte_call(tokens: &[Token]) -> bool {
    let Some(name) = tokens.first() else {
        return false;
    };
    let Some(open) = tokens.get(1) else {
        return false;
    };
    name.kind == TokenKind::Identifier
        && name.lexeme == "range_dual_byte"
        && open.kind == TokenKind::LParen
}

fn source_multi_range_check_arguments(
    tokens: &[Token],
    arguments: Vec<(usize, usize)>,
) -> Option<SourceMultiRangeCheckArguments> {
    let mut out = PartialSourceMultiRangeCheckArguments::default();
    for (index, range) in arguments.into_iter().enumerate() {
        let argument = split_named_argument(tokens, range);
        match argument.name.as_deref() {
            Some("expression") => out.expression = Some(argument.value_range),
            Some("min1") => out.min1 = Some(argument.value_range),
            Some("max1") => out.max1 = Some(argument.value_range),
            Some("min2") => out.min2 = Some(argument.value_range),
            Some("max2") => out.max2 = Some(argument.value_range),
            Some("range_sel") => out.range_selector = Some(argument.value_range),
            Some("sel") => out.selector = Some(argument.value_range),
            Some("predefined") => out.predefined = Some(argument.value_range),
            Some(_) => return None,
            None => match index {
                0 => out.expression = Some(argument.value_range),
                1 => out.min1 = Some(argument.value_range),
                2 => out.max1 = Some(argument.value_range),
                3 => out.min2 = Some(argument.value_range),
                4 => out.max2 = Some(argument.value_range),
                5 => out.range_selector = Some(argument.value_range),
                6 => out.selector = Some(argument.value_range),
                7 => out.predefined = Some(argument.value_range),
                _ => return None,
            },
        }
    }
    out.finish()
}

fn source_range_dual_byte_arguments(
    tokens: &[Token],
    arguments: Vec<(usize, usize)>,
) -> Option<SourceRangeDualByteArguments> {
    let mut out = PartialSourceRangeDualByteArguments::default();
    for (index, range) in arguments.into_iter().enumerate() {
        let argument = split_named_argument(tokens, range);
        match argument.name.as_deref() {
            Some("byte_a") => out.byte_a = Some(argument.value_range),
            Some("byte_b") => out.byte_b = Some(argument.value_range),
            Some("sel") => out.selector = Some(argument.value_range),
            Some(_) => return None,
            None => match index {
                0 => out.byte_a = Some(argument.value_range),
                1 => out.byte_b = Some(argument.value_range),
                2 => out.selector = Some(argument.value_range),
                _ => return None,
            },
        }
    }
    out.finish()
}

fn source_range_check_arguments(
    tokens: &[Token],
    arguments: Vec<(usize, usize)>,
) -> Option<SourceRangeCheckArguments> {
    let mut out = PartialSourceRangeCheckArguments::default();
    for (index, range) in arguments.into_iter().enumerate() {
        let argument = split_named_argument(tokens, range);
        match argument.name.as_deref() {
            Some("expression") => out.expression = Some(argument.value_range),
            Some("min") => out.min = Some(argument.value_range),
            Some("max") => out.max = Some(argument.value_range),
            Some("sel") => out.selector = Some(argument.value_range),
            Some("predefined") => out.predefined = Some(argument.value_range),
            Some(_) => return None,
            None => match index {
                0 => out.expression = Some(argument.value_range),
                1 => out.min = Some(argument.value_range),
                2 => out.max = Some(argument.value_range),
                3 => out.selector = Some(argument.value_range),
                4 => out.predefined = Some(argument.value_range),
                _ => return None,
            },
        }
    }
    out.finish()
}

#[derive(Debug, Clone)]
struct SourceRangeCheckArgument {
    name: Option<String>,
    value_range: (usize, usize),
}

fn split_named_argument(tokens: &[Token], range: (usize, usize)) -> SourceRangeCheckArgument {
    if range.0 + 2 <= range.1
        && tokens[range.0].kind == TokenKind::Identifier
        && tokens[range.0 + 1].kind == TokenKind::Colon
    {
        return SourceRangeCheckArgument {
            name: Some(tokens[range.0].lexeme.clone()),
            value_range: (range.0 + 2, range.1),
        };
    }
    SourceRangeCheckArgument {
        name: None,
        value_range: range,
    }
}

fn source_static_integer_argument(
    inputs: &SourceLookupInputs<'_>,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
) -> Option<i128> {
    let expression =
        parse_source_expression(inputs.module.source_name.clone(), line, tokens, range)?;
    let value = evaluate_source_static_expression(inputs.program, &expression, inputs.values)
        .or_else(|| {
            let mut values = inputs.constant_values.clone();
            values.extend(inputs.values.clone());
            evaluate_source_static_expression(inputs.program, &expression, &values)
        })?;
    static_value_integer(&value)
}

fn parse_source_expression(
    source_name: String,
    line: &str,
    tokens: &[Token],
    range: (usize, usize),
) -> Option<lzvm_pil::Expression> {
    let source = SourceFile {
        contents: line.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::new(),
        source_name,
    };
    let (expression, consumed) = parse_expression_tokens(tokens, range.0, range.1, &source).ok()?;
    (consumed == range.1).then_some(expression)
}

fn source_argument_text(line: &str, tokens: &[Token], range: (usize, usize)) -> Option<String> {
    if range.0 >= range.1 {
        return None;
    }
    let start = tokens.get(range.0)?.start;
    let end = tokens.get(range.1.checked_sub(1)?)?.end;
    line.get(start..end)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn top_level_argument_ranges(
    tokens: &[Token],
    open_index: usize,
    close_index: usize,
) -> Option<Vec<(usize, usize)>> {
    if open_index >= close_index {
        return None;
    }
    let mut ranges = Vec::new();
    let mut start = open_index + 1;
    let mut depth = 0_i32;
    for (index, token) in tokens
        .iter()
        .enumerate()
        .take(close_index)
        .skip(open_index + 1)
    {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => depth -= 1,
            TokenKind::Comma if depth == 0 => {
                if start == index {
                    return None;
                }
                ranges.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    if depth != 0 {
        return None;
    }
    if start < close_index {
        ranges.push((start, close_index));
    }
    Some(ranges)
}
