use std::collections::BTreeMap;
use std::path::PathBuf;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, lex_source, parse_expression,
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, FunctionDeclaration,
    FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind, SourceFile,
    SourceProgram, SourceProgramModule, TokenKind, UnaryOperator,
};

pub(crate) fn source_scalar_constant_values(
    program: &SourceProgram,
    row_count: u64,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = BTreeMap::from([
        (
            "BITS".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count.trailing_zeros())),
        ),
        (
            "N".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count)),
        ),
    ]);
    let declarations = program
        .modules
        .iter()
        .flat_map(|module| module.constants.iter())
        .collect::<Vec<_>>();
    let mut resolved = vec![false; declarations.len()];

    loop {
        let mut progressed = false;
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved[index] {
                continue;
            }
            if !declaration.array_dims.is_empty() || values.contains_key(&declaration.name) {
                resolved[index] = true;
                progressed = true;
                continue;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                continue;
            };
            let Some(value) = evaluate_source_static_expression(program, expression, &values)
            else {
                continue;
            };
            values.insert(declaration.name.clone(), value);
            resolved[index] = true;
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    values
}

const STATIC_LOOP_LIMIT: usize = 10_000;

fn evaluate_source_static_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<FixedFileTemplateValue> {
    evaluate_fixed_file_template_value_expression_with_values(expression, values).or_else(|| {
        let env = integer_values(values);
        evaluate_static_i128(program, expression, &env).map(FixedFileTemplateValue::Integer)
    })
}

fn integer_values(values: &BTreeMap<String, FixedFileTemplateValue>) -> BTreeMap<String, i128> {
    values
        .iter()
        .filter_map(|(name, value)| match value {
            FixedFileTemplateValue::Integer(value) => Some((name.clone(), *value)),
            _ => None,
        })
        .collect()
}

fn evaluate_static_i128(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => parse_i128(value),
        ExpressionKind::Name(name) => values.get(name).copied(),
        ExpressionKind::Group(inner) => evaluate_static_i128(program, inner, values),
        ExpressionKind::Unary { op, expr } => {
            let value = evaluate_static_i128(program, expr, values)?;
            match op {
                UnaryOperator::Plus => Some(value),
                UnaryOperator::Minus => value.checked_neg(),
                UnaryOperator::Not => Some(static_bool(value == 0)),
                _ => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            evaluate_static_binary(program, *op, left, right, values)
        }
        ExpressionKind::Call { callee, args } if args.is_empty() => {
            let name = expression_name(callee)?;
            evaluate_static_zero_arg_function(program, name, values)
        }
        _ => None,
    }
}

fn evaluate_static_binary(
    program: &SourceProgram,
    op: BinaryOperator,
    left: &Expression,
    right: &Expression,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    if op == BinaryOperator::LogicalAnd {
        let left = evaluate_static_i128(program, left, values)?;
        if left == 0 {
            return Some(0);
        }
        return Some(static_bool(
            evaluate_static_i128(program, right, values)? != 0,
        ));
    }
    if op == BinaryOperator::LogicalOr {
        let left = evaluate_static_i128(program, left, values)?;
        if left != 0 {
            return Some(1);
        }
        return Some(static_bool(
            evaluate_static_i128(program, right, values)? != 0,
        ));
    }

    let left = evaluate_static_i128(program, left, values)?;
    let right = evaluate_static_i128(program, right, values)?;
    match op {
        BinaryOperator::Add => left.checked_add(right),
        BinaryOperator::Subtract => left.checked_sub(right),
        BinaryOperator::Multiply => left.checked_mul(right),
        BinaryOperator::Divide if right != 0 => Some(left / right),
        BinaryOperator::Modulo if right != 0 => Some(left % right),
        BinaryOperator::Power => u32::try_from(right)
            .ok()
            .and_then(|exponent| left.checked_pow(exponent)),
        BinaryOperator::ShiftLeft => u32::try_from(right)
            .ok()
            .and_then(|amount| left.checked_shl(amount)),
        BinaryOperator::ShiftRight => u32::try_from(right)
            .ok()
            .and_then(|amount| left.checked_shr(amount)),
        BinaryOperator::Less => Some(static_bool(left < right)),
        BinaryOperator::LessEqual => Some(static_bool(left <= right)),
        BinaryOperator::Greater => Some(static_bool(left > right)),
        BinaryOperator::GreaterEqual => Some(static_bool(left >= right)),
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => {
            Some(static_bool(left == right))
        }
        BinaryOperator::NotEqual => Some(static_bool(left != right)),
        _ => None,
    }
}

fn static_bool(value: bool) -> i128 {
    if value {
        1
    } else {
        0
    }
}

fn evaluate_static_zero_arg_function(
    program: &SourceProgram,
    name: &str,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    for module in &program.modules {
        let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name && function.parameters.is_empty())
        else {
            continue;
        };
        return evaluate_static_function(program, module, function, values);
    }
    None
}

fn evaluate_static_function(
    program: &SourceProgram,
    module: &SourceProgramModule,
    function: &FunctionDeclaration,
    values: &BTreeMap<String, i128>,
) -> Option<i128> {
    let mut values = values.clone();
    for statement in &function.statements {
        if let StaticFlow::Return(value) =
            execute_static_statement(program, module, statement, &mut values)?
        {
            return Some(value);
        }
    }
    None
}

enum StaticFlow {
    Continue,
    Return(i128),
}

fn execute_static_statement(
    program: &SourceProgram,
    module: &SourceProgramModule,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    match statement.kind {
        FunctionStatementKind::Declaration => {
            execute_static_declaration(program, statement, values)?;
            Some(StaticFlow::Continue)
        }
        FunctionStatementKind::Expression => {
            execute_static_expression_statement(
                program,
                statement.value_expression.as_ref()?,
                values,
            )?;
            Some(StaticFlow::Continue)
        }
        FunctionStatementKind::Return => {
            let value =
                evaluate_static_i128(program, statement.value_expression.as_ref()?, values)?;
            Some(StaticFlow::Return(value))
        }
        FunctionStatementKind::While => {
            let condition = statement.header_expression.as_ref()?;
            let body = statement.body.as_ref()?;
            for _ in 0..STATIC_LOOP_LIMIT {
                if evaluate_static_i128(program, condition, values)? == 0 {
                    return Some(StaticFlow::Continue);
                }
                if let StaticFlow::Return(value) =
                    execute_static_body(program, module, body.start, body.end, values)?
                {
                    return Some(StaticFlow::Return(value));
                }
            }
            None
        }
        _ => None,
    }
}

fn execute_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    match statement.declaration.as_ref()? {
        FunctionStatementDeclaration::Constant(declaration) => {
            let expression = declaration.initializer_expression.as_ref()?;
            let value = evaluate_static_i128(program, expression, values)?;
            values.insert(declaration.name.clone(), value);
        }
        FunctionStatementDeclaration::Variable(declaration) => {
            let expression = declaration.initializer_expression.as_ref()?;
            let value = evaluate_static_i128(program, expression, values)?;
            values.insert(declaration.name.clone(), value);
        }
        FunctionStatementDeclaration::Column(_) => return None,
    }
    Some(())
}

fn execute_static_body(
    program: &SourceProgram,
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    values: &mut BTreeMap<String, i128>,
) -> Option<StaticFlow> {
    let body = module.source.contents.get(start..end)?;
    let body = body
        .trim()
        .strip_prefix('{')
        .and_then(|body| body.strip_suffix('}'))
        .unwrap_or(body);
    let source = SourceFile {
        contents: body.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::new(),
        source_name: module.source_name.clone(),
    };
    execute_static_expression_statements(program, &source, values)?;
    Some(StaticFlow::Continue)
}

fn execute_static_expression_statements(
    program: &SourceProgram,
    source: &SourceFile,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    let tokens = lex_source(&source.contents).ok()?;
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon => {
                index += 1;
            }
            TokenKind::EndOfInput => break,
            _ => {
                let end = next_static_semicolon(&tokens, index)?;
                let (expression, consumed) = parse_expression(source, index, end).ok()?;
                if consumed != end {
                    return None;
                }
                execute_static_expression_statement(program, &expression, values)?;
                index = end + 1;
            }
        }
    }
    Some(())
}

fn next_static_semicolon(tokens: &[lzvm_pil::Token], index: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (cursor, token) in tokens.iter().enumerate().skip(index) {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                depth = depth.checked_add(1)?;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.checked_sub(1)?;
            }
            TokenKind::Semicolon if depth == 0 => return Some(cursor),
            TokenKind::EndOfInput => return None,
            _ => {}
        }
    }
    None
}

fn execute_static_expression_statement(
    program: &SourceProgram,
    expression: &Expression,
    values: &mut BTreeMap<String, i128>,
) -> Option<()> {
    let ExpressionKind::Binary { op, left, right } = &expression.kind else {
        return None;
    };
    let name = expression_name(left)?.to_owned();
    let right = evaluate_static_i128(program, right, values)?;
    let current = values.get(&name).copied().unwrap_or_default();
    let value = match op {
        BinaryOperator::Assign => right,
        BinaryOperator::PlusAssign => current.checked_add(right)?,
        BinaryOperator::MinusAssign => current.checked_sub(right)?,
        BinaryOperator::StarAssign => current.checked_mul(right)?,
        _ => return None,
    };
    values.insert(name, value);
    Some(())
}

fn expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => expression_name(inner),
        _ => None,
    }
}

fn parse_i128(value: &str) -> Option<i128> {
    let value = value.trim().replace('_', "");
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        return i128::from_str_radix(hex, 16)
            .ok()
            .and_then(i128::checked_neg);
    }
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return i128::from_str_radix(hex, 16).ok();
    }
    value.parse::<i128>().ok()
}

pub(crate) fn source_declaration_constant_values(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = base_values.clone();
    let Some(template) = module
        .air_templates
        .iter()
        .find(|template| template.body.start <= start && end <= template.body.end)
    else {
        return values;
    };

    for parameter in &template.parameters {
        if values.contains_key(&parameter.name) {
            continue;
        }
        let Some(expression) = parameter.default_expression.as_ref() else {
            continue;
        };
        let Some(value) =
            evaluate_fixed_file_template_value_expression_with_values(expression, &values)
        else {
            continue;
        };
        values.insert(parameter.name.clone(), value);
    }

    for declaration in &module.constants {
        if declaration.start < template.body.start || declaration.end > template.body.end {
            continue;
        }
        if declaration.end > start
            || !declaration.array_dims.is_empty()
            || values.contains_key(&declaration.name)
        {
            continue;
        }
        let Some(expression) = declaration.initializer_expression.as_ref() else {
            continue;
        };
        let Some(value) =
            evaluate_fixed_file_template_value_expression_with_values(expression, &values)
        else {
            continue;
        };
        values.insert(declaration.name.clone(), value);
    }

    values
}
